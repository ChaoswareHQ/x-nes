# Debugging Guide

This guide covers techniques for debugging NES emulation issues in x-nes, from CPU bugs to PPU rendering glitches, mapper problems, and APU timing.

## Quick Diagnostic Commands

### Run ROM headless (check if it boots)
```bash
cargo run --example main -- "path/to/rom.nes"
```

Watch the PC values. If they cycle predictably and you see `Frame complete` messages, the game is executing code and generating frames.

### Run with window (see visuals)
```bash
cargo run --example window -- "path/to/rom.nes"
```

### Run test suite
```bash
cargo test                          # Unit tests
cargo test --test accuracy_coin     # AccuracyCoin (comprehensive)
```

## Debugging CPU Issues

### Inspect CPU State

Add this to the main loop for per-instruction tracing:

```rust
println!(
    "{:04X}  A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}  CYC:{}",
    cpu.pc(), cpu.a(), cpu.x(), cpu.y(), cpu.sr(), cpu.st(),
    bus.cpu_cycle
);
```

### Check Reset Vectors

The NES boots by reading the reset vector at `$FFFC-$FFFD`. Verify these addresses contain valid ROM data:

```rust
let lo = bus.read(0xFFFC);
let hi = bus.read(0xFFFD);
println!("Reset vector: ${:02X}{:02X} → ${:04X}", hi, lo, (hi as u16) << 8 | lo as u16);
```

If the reset vector points to `$0000` or a non-ROM address, the ROM isn't loading correctly.

### Trace PC history

Keep a rolling window of last N PCs to see code flow:

```rust
let mut pc_history = [0u16; 64];
let mut pc_idx = 0;

// In the tick loop:
pc_history[pc_idx % 64] = cpu.pc();
pc_idx += 1;

// Print on crash/detection:
for i in 0..64.min(pc_idx) {
    let idx = (pc_idx - 1 - i) % 64;
    println!("  [-{}] PC=${:04X}", i, pc_history[idx]);
}
```

### Detect Infinite Loops

If the game hangs, it's often in a tight loop. Watch for the same PC repeating:

```rust
let mut last_pc = 0u16;
let mut repeat_count = 0u32;

// In tick:
if cpu.pc() == last_pc {
    repeat_count += 1;
    if repeat_count > 1_000_000 {
        eprintln!("HANG DETECTED at PC=${:04X}", cpu.pc());
        break;
    }
} else {
    repeat_count = 0;
    last_pc = cpu.pc();
}
```

### Illegal Opcodes

If the CPU hits an unofficial opcode, it may crash or behave unexpectedly. x-nes implements all official and unofficial opcodes. Verify the opcode table covers the full 256 entries:

```rust
// Check if any slot is missing:
for i in 0..256 {
    // TABLE[i] should always be a valid function pointer
}
```

## Debugging PPU / Graphics Issues

See the [Graphics Debugging](graphics-debugging.md) chapter for a dedicated guide.

### Quick PPU State Dump

```rust
fn dump_ppu(ppu: &Ppu) {
    println!("Scanline: {}  Cycle: {}  Frame: {}",
        ppu.scanline, ppu.cycle, ppu.frame_complete);
    println!("CTRL: {:02X}  MASK: {:02X}  STATUS: {:02X}",
        ppu.ctrl, ppu.mask, ppu.status);
    println!("v: ${:04X}  t: ${:04X}  fine_x: {}  w: {}",
        ppu.v, ppu.t, ppu.fine_x, ppu.w);
    println!("NMI: output={} latched={} vblank={} deferred={}",
        ppu.nmi_output, ppu.nmi_latched,
        ppu.nmi_from_vblank, ppu.nmi_deferred_pending);
}
```

### Dump Frame to PPM

Save the frame buffer as a viewable image for offline analysis:

```rust
fn save_ppm(frame: &[u8; 61440], path: &str, palette: &[u32; 64]) {
    use std::io::Write;
    let mut f = std::fs::File::create(path).unwrap();
    writeln!(f, "P6\n256 240\n255").unwrap();
    for y in 0..240 {
        for x in 0..256 {
            let idx = y * 256 + x;
            let color = palette[(frame[idx] & 0x3F) as usize];
            let r = ((color >> 16) & 0xFF) as u8;
            let g = ((color >> 8) & 0xFF) as u8;
            let b = (color & 0xFF) as u8;
            f.write_all(&[r, g, b]).unwrap();
        }
    }
    // View with: ffplay path.ppm or convert to PNG
}
```

## Debugging Mapper Issues

### Verify Mapper Detection

```rust
let rom = Rom::new(&data).unwrap();
println!("Mapper: {}  Mirroring: {}  PRG: {}KB  CHR: {}KB  CHR-RAM: {}",
    rom.mapper_id, rom.mirroring,
    rom.prg.len()/1024, rom.chr.len()/1024, rom.has_chr_ram);
```

### Log Mapper Register Writes

Add conditional logging for your mapper:

```rust
// In MMC5 cpu_write, add:
if addr >= 0x5100 && addr <= 0x5117 {
    eprintln!("MMC5 write ${:04X} = ${:02X}  (PC=${:04X})", addr, val, cpu_pc);
}
```

This reveals what the game writes to configuration registers.

### Check Bank Mapping

Verify PRG banks are being set correctly:

```rust
fn dump_mmc5_banks(mmc5: &Mmc5) {
    println!("PRG mode: {}  Banks: {:02X} {:02X} {:02X} {:02X}",
        mmc5.prg_mode,
        mmc5.prg_reg[0], mmc5.prg_reg[1],
        mmc5.prg_reg[2], mmc5.prg_reg[3]);
    println!("CHR mode: {}  NT mapping: {:02X}",
        mmc5.chr_mode, mmc5.nt_mapping_reg);
    println!("ExRAM mode: {}  IRQ: scanline={} enabled={} pending={}",
        mmc5.ex_ram_mode, mmc5.irq_scanline,
        mmc5.irq_enable, mmc5.irq_pending_flag);
}
```

### Verify PRG RAM Protection Logic

A common bug is inverted PRG RAM protection. Check:

```rust
// After reset ($5102=0, $5103=0), writes to $6000-$7FFF should be ALLOWED
// When $5102=2, $5103=1, writes should be BLOCKED

// Correct:
fn prg_ram_is_protected(&self) -> bool {
    self.prg_ram_protect1 == 0x02 && self.prg_ram_protect2 == 0x01
}
// Call site: if !self.prg_ram_is_protected() { /* allow write */ }
```

## Debugging IRQ Issues

IRQ bugs cause screen splits to be at the wrong position, audio glitches, or game hangs.

### Log IRQ Events

```rust
// In notify_scanline:
if scanline < 240 && irq_enabled && scanline as u8 == irq_scanline {
    eprintln!("MMC5 IRQ FIRED at scanline {} (target={})", scanline, irq_scanline);
}
```

### Verify Scanline Counter

Check that the IRQ fires at the expected scanline. For Castlevania III, the status bar split should be at a consistent scanline (around 200-210 depending on the screen).

### Check IRQ Acknowledgment

After the game handles an IRQ, it must read `$5204` (or write `$5204` with bit 7 clear) to acknowledge. If acknowledgment doesn't clear `irq_pending_flag`, the CPU will re-enter the IRQ handler immediately, causing a crash.

## Debugging APU / Audio Issues

### Check APU Sample Output

Log audio samples to detect silence, clipping, or DC offset:

```rust
let mut max_sample = 0i16;
let mut min_sample = 0i16;
for &s in &apu.audio_samples[..apu.sample_count] {
    max_sample = max_sample.max(s);
    min_sample = min_sample.min(s);
}
eprintln!("Audio: {} samples, range [{}, {}]", apu.sample_count, min_sample, max_sample);
```

### Verify APU Channel States

```rust
fn dump_apu(apu: &Apu) {
    println!("Pulse1: enabled={} duty={} vol={} timer={} len={}",
        apu.p1.enabled, apu.p1.duty, apu.p1.vol,
        apu.p1.timer_val, apu.p1.length_counter);
    println!("Pulse2: enabled={} duty={} vol={} timer={} len={}",
        apu.p2.enabled, apu.p2.duty, apu.p2.vol,
        apu.p2.timer_val, apu.p2.length_counter);
    println!("Triangle: enabled={} timer={} lin_ctr={}",
        apu.triangle.enabled, apu.triangle.timer_val,
        apu.triangle.linear_counter);
    println!("Noise: enabled={} vol={} mode={} period={}",
        apu.noise.enabled, apu.noise.vol,
        apu.noise.mode, apu.noise.period_index);
}
```

## Test ROMs for Validation

| ROM | Tests |
|-----|-------|
| `nestest.nes` | CPU instructions (all official + unofficial) |
| `instr_test-v5/` | Per-instruction CPU tests |
| `blargg_ppu_tests/` | PPU timing, VBlank, NMI |
| `AccuracyCoin` | Comprehensive (CPU, PPU, APU, DMA, controllers) |
| Any NROM game | Basic mapper + PPU integration |

## Crashes: First Steps

When a game crashes, check in this order:

1. **PC valid?** Is it in the ROM range ($8000-$FFFF)?
2. **Opcode valid?** Print the opcode at PC
3. **Stack overflow?** Check if SP is near 0
4. **IRQ storm?** Check if `poll_irq()` returns true repeatedly
5. **Mapper config?** Dump mapper register state
6. **NMI timing?** Check if NMI fires at the right scanline

## Quick Crash Checklist

```
□ PC = ${:04X} — in ROM range?
□ Opcode at PC = ${:02X} — valid instruction?
□ Stack SP = ${:02X} — not near overflow?
□ Mapper ID = {} — correct for this ROM?
□ PPU scanline {} cycle {} — stuck in VBlank?
□ IRQ pending? {} — stuck in interrupt handler?
```
