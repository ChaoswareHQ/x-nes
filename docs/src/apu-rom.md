# 5. Audio & ROM Loading

## 5.1 Audio Processing Unit (APU)

The NES APU generates sound through five channels:

| Channel | Type | Characteristics |
|---------|------|-----------------|
| Pulse 1 | Square wave | 4 duty cycles, sweep, envelope |
| Pulse 2 | Square wave | 4 duty cycles, sweep, envelope |
| Triangle | Triangle wave | Linear counter, no volume control |
| Noise | Pseudo-random | 16-bit shift register, 16 frequencies |
| DMC | Delta Modulation | 1-bit samples, DMA from ROM |

### 5.1.1 APU Registers

The APU occupies $4000-$4017:

```
$4000-$4003:   Pulse 1 (duty/volume, sweep, timer low, timer high/len)
$4004-$4007:   Pulse 2
$4008-$400B:   Triangle
$400C-$400F:   Noise
$4010-$4013:   DMC
$4015:         Status register (read = channel activity, write = enable)
$4017:         Frame counter (mode, IRQ disable)
```

The x-nes APU is a minimal stub that tracks writes but doesn't generate samples:

```rust
pub struct Apu {
    pub cycles: u64,
}

impl Apu {
    pub fn new() -> Self { Self { cycles: 0 } }

    pub fn tick(&mut self, cpu_cycles: u8) {
        self.cycles += cpu_cycles as u64;
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x4015 => {
                let s = 0;  // TODO: read channel status
                s
            }
            _ => 0,
        }
    }

    pub fn write(&mut self, addr: u16, _val: u8) {
        match addr {
            0x4000..=0x4013 | 0x4017 => { /* TODO: channel control */ }
            0x4015 => { /* TODO: channel enable */ }
            _ => {}
        }
    }
}
```

### 5.1.2 APU Frame Sequencer

The APU operates on a frame sequencer that runs at 60 Hz (for NTSC). Each frame step updates:

- Envelope units (every step)
- Sweep units (every other step)
- Length counters (every step or every other step, depending on mode)

The frame sequencer is driven by writing $4017 or by the CPU's frame counter.

## 5.2 ROM Loading

NES games are distributed as ROM files using the **iNES** format. x-nes implements a parser for this format.

### 5.2.1 iNES Header Format

Every iNES ROM starts with a 16-byte header:

```
Offset  Size  Field
0       4     Identifier: "NES^Z" ($4E $45 $53 $1A)
4       1     PRG-ROM size (16KB units)
5       1     CHR-ROM size (8KB units)
6       1     Flags 6:
                Bit 0: Mirroring (0=horizontal, 1=vertical)
                Bit 1: Battery-backed SRAM
                Bit 2: Trainer present (512 bytes after header)
                Bit 3: Four-screen VRAM
                Bits 7-4: Mapper nibble (low)
7       1     Flags 7:
                Bits 7-4: Mapper nibble (high)
                Bit 0: VS Unisystem
                Bit 1: PlayChoice-10
8       1     PRG-RAM size (8KB units)
9       1     Flags 9: TV system (0=NTSC, 1=PAL)
10      1     Flags 10: TV system, PRG-RAM
11-15   5     Unused padding
```

### 5.2.2 The Rom Struct

```rust
pub struct Rom {
    pub prg: [u8; 0x8000],  // PRG-ROM data (up to 32KB)
    pub chr: [u8; 0x2000],  // CHR-ROM data (up to 8KB)
    pub mapper: u8,          // Mapper number
    pub mirroring: u8,       // 0=horizontal, 1=vertical
    pub has_chr_ram: bool,   // True if no CHR-ROM (use writable RAM)
}
```

### 5.2.3 Parsing the ROM

```rust
impl Rom {
    pub fn new(data: &[u8]) -> Option<Self> {
        // Validate header
        if data.len() < 16 || data[0..4] != [0x4E, 0x45, 0x53, 0x1A] {
            return None;
        }

        let prg_16kb = data[4] as usize;   // number of 16KB PRG banks
        let chr_8kb = data[5] as usize;    // number of 8KB CHR banks
        let flags6 = data[6];
        let flags7 = data[7];

        let mapper = (flags7 & 0xF0) | (flags6 >> 4);
        let mirroring = flags6 & 0x01;
        let has_chr_ram = chr_8kb == 0;

        // Trainer is 512 bytes between header and ROM data
        let header_size = if flags6 & 0x04 != 0 { 16 + 512 } else { 16 };

        // Copy PRG-ROM
        let prg_size = prg_16kb * 0x4000;
        let mut prg = [0u8; 0x8000];
        let prg_src = &data[header_size..header_size + prg_size];
        if prg_size <= 0x8000 {
            prg[..prg_size].copy_from_slice(prg_src);
            // Mirror 16KB to 32KB if needed
            if prg_size == 0x4000 {
                prg[0x4000..0x8000].copy_from_slice(prg_src);
            }
        }

        // Copy CHR-ROM (or leave as zeros for CHR-RAM)
        let chr_start = header_size + prg_size;
        let chr_size = if chr_8kb == 0 { 0x2000 } else { chr_8kb * 0x2000 };
        let mut chr = [0u8; 0x2000];
        if chr_size > 0 {
            let chr_src = &data[chr_start..chr_start + chr_size.min(0x2000)];
            chr[..chr_src.len()].copy_from_slice(chr_src);
        }

        Some(Self { prg, chr, mapper, mirroring, has_chr_ram })
    }
}
```

### 5.2.4 NROM (Mapper 0)

NROM is the simplest mapper and the only one implemented. In NROM:

- **16KB PRG-ROM**: Mirrored to fill 32KB ($8000-$FFFF)
- **32KB PRG-ROM**: Used as-is ($8000-$FFFF)
- **8KB CHR-ROM**: Connected directly to PPU pattern tables
- **No CHR-ROM**: PPU uses CHR-RAM (writable memory for pattern tables)

The `Bus` reads PRG data using the mod mirroring pattern:

```rust
fn read_prg(&self, addr: u16) -> u8 {
    let idx = ((addr - 0x8000) as usize) % self.prg.len();
    self.prg[idx]
}
```

For a 16KB ROM (16384 bytes):
- `$8000` → index 0
- `$BFFF` → index 16383
- `$C000` → index 16384 % 16384 = 0 (mirror!)
- `$FFFF` → index 32767 % 16384 = 16383 (mirror!)

## 5.3 How to Load a ROM

```rust
use nes::{rom::Rom, bus::Bus, cpu::CpuRp2A03, lib::{tick, reset}};

// Read ROM file
let data = std::fs::read("game.nes").unwrap();

// Parse ROM header
let rom = Rom::new(&data).unwrap();

// Create CPU and Bus
let mut cpu = CpuRp2A03::new(0);
let mut bus = Bus::new(&rom.prg, &rom.chr);

// Reset (read reset vector from ROM)
reset(&mut cpu, &mut bus);

// Emulate
loop {
    tick(&mut cpu, &mut bus);
    if bus.ppu.frame_complete {
        // Display frame from bus.ppu.frame
        bus.ppu.frame_complete = false;
    }
}
```

## Summary

- The APU has 5 sound channels controlled by 21 registers
- x-nes currently implements APU register stubs
- iNES ROMs have a 16-byte header followed by PRG and CHR data
- NROM (mapper 0) is the simplest cartridge type
- 16KB PRG-ROM is mirrored to fill 32KB
- ROM data is borrowed, not owned — no allocation needed
