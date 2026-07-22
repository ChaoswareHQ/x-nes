# MMC5 Deep Dive

The MMC5 (Nintendo MMC5, mapper 5) is the most complex mapper ever produced for the NES. Used by only a handful of games including **Castlevania III**, it provides features far beyond any other mapper.

## MMC5 Features at a Glance

| Feature | Description |
|---------|-------------|
| PRG ROM | Up to 2MB (1024 × 8KB banks) via 4 switching modes |
| CHR ROM | Up to 1MB (1024 × 1KB banks) with separate BG/sprite banking |
| ExRAM | 1024 bytes with 4 operating modes |
| Scanline IRQ | For split-screen effects (status bars) |
| Hardware multiplier | 8×8→16 multiply at $5205/$5206 |
| Fill mode | Nametable auto-filling |
| Vertical split | Independent scrolling for two screen halves |
| Expansion audio | 2 extra pulse channels (PCM-capable) |

## Register Map

### Core Configuration Registers

| Register | Function |
|----------|----------|
| `$5100` | PRG mode (0-3) |
| `$5101` | CHR mode (0-3) |
| `$5102` | PRG RAM protect 1 |
| `$5103` | PRG RAM protect 2 |
| `$5104` | ExRAM mode (0-3) |
| `$5105` | Nametable mapping (2 bits per NT) |
| `$5106` | Fill-mode tile |
| `$5107` | Fill-mode attribute (bits 0-1) |

### Bank Registers

| Register | Function |
|----------|----------|
| `$5113` | PRG RAM bank select (0-7) |
| `$5114-$5117` | PRG ROM bank registers (8KB each) |
| `$5120-$5127` | CHR sprite banks (1KB each, 8 total) |
| `$5128-$512B` | CHR background banks (1KB each, 4 total) |
| `$5130` | CHR upper bits (bits 8-9 of all CHR banks) |

### IRQ & Multiplier

| Register | Function |
|----------|----------|
| `$5203` | IRQ scanline target |
| `$5204` | IRQ status (read) / enable (write) |
| `$5205` | Multiplier: write operand A (read: result low) |
| `$5206` | Multiplier: write operand B (read: result high) |

### ExRAM Access

| Address | Function |
|---------|----------|
| `$5C00-$5FFF` | ExRAM read/write (mode-dependent) |

## PRG Banking

MMC5 offers 4 PRG modes, selected by `$5100` bits 0-1:

### Mode 0: 32KB (like NROM)
The entire `$8000-$FFFF` range comes from one 32KB-aligned bank. Register `$5117` bits 2-6 select the bank (×1 multiplier).

```
$8000-$FFFF → PRG register $5117, bits 2-6 (32KB granularity)
```

### Mode 1: 2×16KB (like UxROM)
Two independent 16KB regions:

```
$8000-$BFFF → PRG register $5115, bits 1-6 (16KB granularity)
$C000-$FFFF → PRG register $5117, bits 1-6 (16KB granularity)
```

### Mode 2: 16KB + 8KB + 8KB
One 16KB bank plus two 8KB banks:

```
$8000-$BFFF → PRG register $5115, bits 1-6 (16KB granularity)
$C000-$DFFF → PRG register $5116, bits 0-6 (8KB granularity)
$E000-$FFFF → PRG register $5117, bits 0-6 (8KB granularity)
```

### Mode 3: 4×8KB
Four independent 8KB banks — this is the most common mode:

```
$8000-$9FFF → PRG register $5114
$A000-$BFFF → PRG register $5115
$C000-$DFFF → PRG register $5116
$E000-$FFFF → PRG register $5117
```

Bank registers are masked to 7 bits (`& 0x7F`), allowing up to 128 8KB banks = 1MB PRG ROM.

### Initial Bank Setup

On power-up, the last two 8KB banks are guaranteed to be at `$C000-$FFFF`:

```rust
let prg_reg = [
    0,                                      // $8000: bank 0
    1,                                      // $A000: bank 1
    if prg8_count >= 2 { prg8_count - 2 } else { 0 },  // $C000: second-to-last
    if prg8_count >= 1 { prg8_count - 1 } else { 0 },  // $E000: last (reset vector)
];
```

## CHR Banking

MMC5 has **separate CHR banking for background and sprites**. The PPU tells the mapper whether it's fetching background or sprite data via the M2/RW timing signals. In x-nes, this is modeled with `chr_fetch_bg`:

```rust
fn set_chr_fetch_bg(&mut self)     { self.chr_fetch_bg = true; }
fn set_chr_fetch_sprite(&mut self) { self.chr_fetch_bg = false; }
```

CHR mode (from `$5101`) determines granularity:

### Mode 0: 8KB
- BG: `chr_bg_reg[3] & 0xFC` selects 4KB page (×4 multiplier)
- Sprites: `chr_sprite_reg[7] & 0xF8` selects 8KB page (×8 multiplier)

### Mode 1: 4KB
- BG: `chr_bg_reg[3] & 0xFC` selects 4KB page
- Sprites: `chr_sprite_reg[3]` or `chr_sprite_reg[7]` select 4KB pages (split at $1000)

### Mode 2: 2KB
- BG: `chr_bg_reg[1]`, `chr_bg_reg[3]` each select 2KB (4 pages total)
- Sprites: `chr_sprite_reg[1]`, `chr_sprite_reg[3]`, `chr_sprite_reg[5]`, `chr_sprite_reg[7]` select 2KB each

### Mode 3: 1KB (Castlevania III uses this)
- BG: `chr_bg_reg[0..3]` select 1KB each
- Sprites: `chr_sprite_reg[0..7]` select 1KB each

The CHR upper bits (`$5130`) provide bits 8-9 of all CHR bank numbers, extending the addressable range to 1024 1KB banks = 1MB CHR ROM.

## ExRAM

The MMC5 has 1024 bytes of extra RAM (`ex_ram`). Its function depends on `$5104`:

### Mode 0: Extra Nametable
ExRAM appears as a 5th physical nametable. The `$5105` register can map any of the 4 NT slots to ExRAM (source 2):

```
$5105 bits for each NT:
  00 = CIRAM A (internal VRAM page 0)
  01 = CIRAM B (internal VRAM page 1)
  10 = ExRAM (mapper's extended RAM)
  11 = Fill mode
```

### Mode 1: Extended Attributes (⭐ Castlevania III uses this)
Each byte in ExRAM corresponds to a background tile (same index as nametable). The ExRAM byte provides:
- **Bits 0-5:** 4KB CHR page (64 pages = 256KB CHR)
- **Bits 6-7:** Palette group (0-3) — replaces the attribute table for this tile

This allows **every background tile to have its own CHR bank and palette**, completely bypassing the standard attribute table! This is how Castlevania III achieves its rich, colorful backgrounds.

```rust
// In ppu_read, ExRAM mode 1 background fetch:
let bank = (u16::from(extended_chr_bank & 0x3F) << 2) + bg_slot;
// extended_chr_bank = 6-bit 4KB page from ExRAM
// bg_slot = 2-bit 1KB sub-slot within the 4KB page
```

### Mode 2: CPU RAM
ExRAM is accessible only by the CPU at `$5C00-$5FFF` for both read and write. Not mapped to PPU space.

### Mode 3: CPU Read-Only
Same as mode 2, but CPU writes are ignored. Useful for read-only lookup tables.

### ExRAM Access Control

```rust
// CPU $5C00-$5FFF reads:
if ex_ram_mode >= 2 {     // Only modes 2 and 3
    ex_ram[addr & 0x3FF]
} else { 0 }              // Modes 0/1: open bus

// CPU $5C00-$5FFF writes:
if ex_ram_mode < 3 {      // Modes 0, 1, 2
    ex_ram[addr & 0x3FF] = val;
}                          // Mode 3: writes ignored
```

## PPU Nametable Mapping (`$5105`)

Unlike other mappers that just pick between horizontal and vertical mirroring, MMC5 can independently map each of the 4 nametables (NT0-NT3) to any of 4 sources:

| Bits | Nametable | Source |
|------|-----------|--------|
| 1-0 | NT0 ($2000) | 0=CIRAM_A, 1=CIRAM_B, 2=ExRAM, 3=Fill |
| 3-2 | NT1 ($2400) | Same encoding |
| 5-4 | NT2 ($2800) | Same encoding |
| 7-6 | NT3 ($2C00) | Same encoding |

Example: `$5105 = 0x44` = vertical mirroring (NT0→A, NT1→B, NT2→A, NT3→B)

```rust
fn nt_mapping(&self) -> u8 { self.nt_mapping_reg }

// PPU resolves NT reads/writes via nt_index():
fn nt_index(addr: u16, mirroring: u8, nt_mapping: u8) -> (u8, u16) {
    if nt_mapping != 0xFF {  // MMC5 path
        let nt = ((addr >> 10) & 3) as u8;
        let source = (nt_mapping >> (nt * 2)) & 0x03;
        (source, addr & 0x03FF)
    } else {
        // Standard mirroring path
    }
}
```

## Fill Mode (source 3)

When a nametable slot is mapped to fill mode (source 3), the PPU always returns the same data regardless of address:
- **Tile areas:** Returns `fill_tile` (`$5106`)
- **Attribute areas:** Returns `(fill_attr & 3) * 0x55` (`$5107`)

This creates a solid-color background with a single repeating tile — useful for status bars, menus, or solid-color areas without using up nametable space.

## Scanline IRQ

MMC5 provides a scanline-based IRQ for split-screen effects. Castlevania III uses this for its status bar at the bottom of the screen.

```
$5203 write: Set IRQ target scanline (0-239)
$5204 write: Bit 7 enables IRQ, also acknowledges pending IRQ
$5204 read:  Bit 7 = IRQ pending, Bit 6 = in-frame, Bits 5-0 = current scanline
```

The PPU calls `notify_scanline()` at the start of each scanline:

```rust
fn notify_scanline(&mut self, scanline: u16) {
    if scanline < 240 {
        // Store scanline counter in status bits 0-5
        self.irq_status = (self.irq_status & 0xC0) | (scanline as u8 & 0x3F);
        self.irq_status |= 0x40;  // in-frame flag

        // Fire IRQ when counter matches target
        if self.irq_enabled && scanline as u8 == self.irq_scanline {
            self.irq_status |= 0x80;  // IRQ pending
            self.irq_pending_flag = true;
        }
    } else {
        self.irq_status &= !0x40;  // Clear in-frame flag
    }
}
```

When the IRQ fires, the game's IRQ handler changes the scroll registers mid-screen, creating a visual split. This is how Castlevania III shows the gameplay area on top and the status bar (health, lives, weapon) on the bottom.

## Hardware Multiplier

Writing to `$5205` or `$5206` triggers an unsigned 8×8→16 multiplication. The result is readable from `$5205` (low byte) and `$5206` (high byte).

```rust
0x5205 => {
    self.mul_a = val;
    self.mul_result = u16::from(self.mul_a) * u16::from(self.mul_b);
}
0x5206 => {
    self.mul_b = val;
    self.mul_result = u16::from(self.mul_a) * u16::from(self.mul_b);
}
```

Castlevania III uses the multiplier for coordinate calculations, screen position math, and game logic.

## PRG RAM Protection

The MMC5 includes PRG RAM at `$6000-$7FFF` with write protection:

```rust
fn prg_ram_is_protected(&self) -> bool {
    // Protected when $5102 == 0x02 AND $5103 == 0x01
    self.prg_ram_protect1 == 0x02 && self.prg_ram_protect2 == 0x01
}
```

Castlevania III uses PRG RAM for save data (game progress, settings). After writing save data, it locks the RAM by writing `$5102=$02` and `$5103=$01` to prevent corruption.

## Expansion Audio

MMC5 adds two extra square-wave channels (PCM-capable) accessed via `$5000-$5015`. These provide richer soundtracks in MMC5 games. x-nes currently accepts register writes but does not synthesize the audio:

```rust
0x5000..=0x5015 => {
    // MMC5 expansion audio registers — accepted but not implemented
}
```

## Debugging MMC5 Games

Key things to check when debugging MMC5 games:

1. **PRG banking:** Verify that `$5100` and `$5114-$5117` are set correctly. The reset vector MUST be in the last 8KB bank.
2. **CHR banking:** Verify `$5101` and `$5120-$512B`. In mode 3, there are 12 separate 1KB CHR bank registers.
3. **ExRAM mode:** Check `$5104`. Mode 1 (extended attributes) requires the PPU to correctly set `chr_fetch_bg`/`chr_fetch_sprite`.
4. **NT mapping:** Check `$5105`. Non-standard NT mappings (ExRAM or fill mode) must be handled by `ppu_read_nt`.
5. **IRQ timing:** Verify `$5203` and `$5204`. The IRQ must fire at exactly the right scanline for correct screen splits.
6. **PRG RAM protection:** Check `$5102/$5103` — inverted logic is a common bug!

### Common Bug: PRG RAM Protection Inverted

A very common MMC5 bug is inverting the PRG RAM write protection logic. After reset, `$5102=0, $5103=0` means "not protected" — writes to `$6000-$7FFF` must be ALLOWED. When `$5102=2, $5103=1`, writes must be BLOCKED.
