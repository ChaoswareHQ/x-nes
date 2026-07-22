# Mappers Overview

NES cartridges contain mapper chips that extend the console's capabilities — bank switching PRG-ROM and CHR-ROM, adding scanline IRQs, extra RAM, and custom nametable features. x-nes currently implements **8 mappers** covering most of the NES library.

## How Mappers Work

The NES CPU can only address 64KB. The cartridge provides the upper 32KB ($8000-$FFFF) as PRG-ROM. For games larger than 32KB, a mapper chip sits between the CPU and the ROM, listening for writes to specific addresses and using those writes as bank-switch commands.

```
CPU writes to $8000-$FFFF
        │
        ▼
┌──────────────┐     Bank select     ┌──────────────┐
│   Mapper      │◄───────────────────│  Game Code    │
│   Chip        │                    │  (writes to   │
│  (bank logic) │                    │   mapper regs)│
└──────┬───────┘                    └──────────────┘
       │
       │  Physical bank address
       ▼
┌──────────────┐
│  PRG-ROM     │   (up to 2MB)
│  CHR-ROM     │
└──────────────┘
```

All mappers implement the `MapperImpl` trait:

```rust
pub trait MapperImpl {
    fn cpu_read(&mut self, addr: u16) -> u8;
    fn cpu_write(&mut self, addr: u16, val: u8);
    fn ppu_read(&mut self, addr: u16) -> u8;
    fn ppu_write(&mut self, addr: u16, val: u8);
    fn mirroring(&self) -> u8;
    fn irq_pending(&self) -> bool;
    fn ack_irq(&mut self);
    fn clock_scanline(&mut self) {}
    fn has_chr_ram(&self) -> bool;

    // MMC5-specific extensions (default no-ops):
    fn notify_scanline(&mut self, _scanline: u16) {}
    fn nt_mapping(&self) -> u8 { 0xFF }
    fn read_nt_ext(&mut self, _addr: u16, _nt_source: u8) -> u8 { 0 }
    fn write_nt_ext(&mut self, _addr: u16, _nt_source: u8, _val: u8) {}
    fn set_chr_fetch_bg(&mut self) {}
    fn set_chr_fetch_sprite(&mut self) {}
    fn set_extended_chr_bank(&mut self, _bank: u8) {}
    fn get_extended_chr_bank(&self) -> u8 { 0 }
    fn get_ex_ram_mode(&self) -> u8 { 0 }
    fn get_fill_tile(&self) -> u8 { 0 }
    fn get_fill_attr(&self) -> u8 { 0 }
    fn read_ex_ram_byte(&mut self, _offset: u16) -> u8 { 0 }
}
```

## Mapper Selection

Mappers are selected automatically from the iNES header's mapper number:

```rust
impl Mapper {
    pub fn from_ines(id: u8, mirroring: u8, prg_data: &[u8], chr_data: &[u8], chr_ram: bool) -> Self {
        match id {
            0 => Self::Nrom(...),    // NROM
            1 => Self::Mmc1(...),    // MMC1 (SxROM)
            2 => Self::UxRom(...),   // UxROM
            3 => Self::Cnrom(...),   // CNROM
            4 => Self::Mmc3(...),    // MMC3 (TxROM)
            5 => Self::Mmc5(...),    // MMC5 (ExROM)
            7 => Self::Axrom(...),   // AxROM
            66 => Self::Gxrom(...),  // GxROM
            _ => Self::Nrom(...),    // Unknown → NROM fallback
        }
    }
}
```

## Implemented Mappers

### Mapper 0 — NROM (`nrom.rs`)
**Used by:** Super Mario Bros., Donkey Kong, Excitebike, Ice Climber

The simplest mapper. No bank switching at all.
- 16KB PRG-ROM: mirrored to fill $8000-$FFFF
- 32KB PRG-ROM: fills entire range
- 8KB CHR-ROM: connected directly to PPU
- No CHR-ROM: uses CHR-RAM (writable pattern tables)

```rust
pub struct Nrom { prg: Vec<u8>, chr: Vec<u8>, mirror: u8, has_chr_ram: bool }
```

PRG reads simply wrap around for 16KB ROMs using `% prg.len()`.

### Mapper 1 — MMC1 (`mmc1.rs`)
**Used by:** The Legend of Zelda, Metroid, Mega Man 2, Castlevania II

Nintendo's first advanced mapper. Features serial register writes (5 writes to build one register value), 4 PRG modes, 2 CHR modes, and variable mirroring.
- **PRG modes:** 32KB switchable, 16KB+16KB fixed, 16KB+16KB swapped
- **CHR modes:** 8KB switchable, 4KB+4KB
- **Mirroring:** Horizontal, Vertical, Single-screen A, Single-screen B
- **PRG RAM:** 8KB at $6000-$7FFF, enable/disable via register bit
- **Serial interface:** 5 consecutive writes with bit 7 determining reset

### Mapper 2 — UxROM (`uxrom.rs`)
**Used by:** Castlevania, Contra, Mega Man, DuckTales

Simple PRG bank switching with a single register.
- 16KB switchable PRG bank at $8000-$BFFF
- 16KB fixed PRG bank (last bank) at $C000-$FFFF
- 8KB CHR-RAM (fixed, writable)
- Horizontal or vertical mirroring (fixed by cartridge)

```rust
pub struct UxRom {
    prg: Vec<u8>, chr: Vec<u8>, mirror: u8,
    prg_bank: u8,    // selects which 16KB bank at $8000
    has_chr_ram: bool,
}
```

### Mapper 3 — CNROM (`cnrom.rs`)
**Used by:** 1942, The Goonies, Gradius

Simple CHR bank switching.
- 32KB PRG-ROM (or 16KB mirrored)
- 8KB switchable CHR bank at PPU $0000-$1FFF
- Horizontal or vertical mirroring

### Mapper 4 — MMC3 (`mmc3.rs`)
**Used by:** Super Mario Bros. 3, Mega Man 3, Kirby's Adventure

The most common advanced mapper. Features dual 8KB PRG banks (one fixed, one switchable), 2KB+1KB CHR granularity, scanline-based IRQ counter, and configurable mirroring.

- **PRG:** Two 8KB switchable banks at $8000 and $A000, two fixed banks (second-to-last and last) at $C000 and $E000. PRG mode swaps which is fixed/switchable.
- **CHR:** 2KB banks at $0000-$0FFF (R0, R1), 1KB banks at $1000-$1FFF (R2-R7). CHR mode swaps these groups.
- **IRQ:** 8-bit down-counter triggered by A12 rising edges (PPU rendering). When it reaches 0, it reloads and fires IRQ.
- **Mirroring:** Dynamic — write to $A000 sets horizontal or vertical.

```rust
pub struct Mmc3 {
    prg: Vec<u8>, chr: Vec<u8>,
    bank_select: u8,     // $8000: bit 6=PRG mode, bit 7=CHR mode, bits 0-2=bank index
    prg_banks: [u8; 4],  // R2,R3 (CHR), R4(R6,R7 PRG) — only indices 2,3 used for PRG
    chr_banks: [u8; 8],  // R0-R7: 2KB+1KB CHR banks
    irq_latch: u8,       // value to reload into counter
    irq_counter: u8,     // current counter value
    irq_enabled: bool,
    irq_reload: bool,
    irq_flag: bool,
    // PRG RAM
    prg_ram: [u8; 0x2000],
    prg_ram_enable: bool,
    prg_ram_write: bool,
}
```

Register writes:
| Address | Even (A0=0) | Odd (A0=1) |
|---------|-------------|------------|
| $8000-$9FFF | Bank Select (`bank_select`) | Bank Data (loads `bank_select & 7`) |
| $A000-$BFFF | Mirroring | PRG RAM protect |
| $C000-$DFFF | IRQ Latch | IRQ Reload |
| $E000-$FFFF | IRQ Disable | IRQ Enable |

### Mapper 5 — MMC5 (`mmc5.rs`) ⭐
**Used by:** Castlevania III, Just Breed, Metal Slader Glory, Uncharted Waters

Nintendo's most powerful mapper. See the [MMC5 Deep Dive](mapper-mmc5.md) for full details.

- **PRG:** 4 modes: 32KB, 2×16KB, 16KB+8KB+8KB, 4×8KB
- **CHR:** 4 modes: 8KB, 4KB+4KB, 2KB×4, 1KB×8 — separate banks for BG and sprites
- **ExRAM:** 1024 bytes with 4 operating modes:
  - Mode 0: Extra nametable (5th screen)
  - Mode 1: Extended attributes (per-tile CHR bank + palette)
  - Mode 2: CPU read/write RAM
  - Mode 3: CPU read-only
- **Scanline IRQ:** for split-screen effects (status bars)
- **Multiplier:** hardware 8×8=16 multiply at $5205/$5206
- **Fill mode:** nametable auto-filling with a single tile
- **Vertical split:** independent scrolling for two screen halves
- **Expansion audio:** 2 extra pulse channels (not yet implemented in x-nes)

### Mapper 7 — AxROM (`axrom.rs`)
**Used by:** Battletoads, Solstice, Marble Madness

Single-screen mirroring with 32KB PRG bank switching.
- 32KB PRG bank at $8000-$FFFF
- 8KB CHR-RAM
- Mirroring selectable (single-screen A or B) via PRG bank write

### Mapper 66 — GxROM (`gxrom.rs`)
**Used by:** Doraemon, Giant Step

Simple dual-bank switching.
- 32KB PRG bank (select via bits 4-5 of register write)
- 8KB CHR bank (select via bits 0-1 of register write)
- Fixed mirroring (hardware-set)

## How the Emulator Dispatches to Mappers

The `Mapper` enum wraps all mapper types:

```rust
pub enum Mapper {
    Nrom(Box<nrom::Nrom>),
    UxRom(Box<uxrom::UxRom>),
    Cnrom(Box<cnrom::Cnrom>),
    Mmc1(Box<mmc1::Mmc1>),
    Mmc3(Box<mmc3::Mmc3>),
    Mmc5(Box<mmc5::Mmc5>),
    Axrom(Box<axrom::Axrom>),
    Gxrom(Box<gxrom::Gxrom>),
    Null,
}
```

Each method in `Mapper` dispatches via `match self` to the correct implementation:

```rust
pub fn cpu_read(&mut self, addr: u16) -> u8 {
    match self {
        Self::Nrom(m) => m.cpu_read(addr),
        Self::Mmc3(m) => m.cpu_read(addr),
        Self::Mmc5(m) => m.cpu_read(addr),
        // ...
        Self::Null => 0,
    }
}
```

## Adding a New Mapper

1. Create `src/mapper/your_mapper.rs` with a struct implementing `MapperImpl`
2. Add `pub mod your_mapper;` to `src/mapper/mod.rs`
3. Add a variant to the `Mapper` enum
4. Add the match arm to each dispatch method
5. Add mapping in `Mapper::from_ines()` based on the mapper ID

Example skeleton:

```rust
use super::MapperImpl;
use alloc::vec::Vec;

pub struct YourMapper {
    prg: Vec<u8>,
    chr: Vec<u8>,
    mirror: u8,
    has_chr_ram: bool,
    // ... your registers
}

impl YourMapper {
    pub fn new(prg: &[u8], chr: &[u8], chr_ram: bool, mirror: u8) -> Self {
        Self {
            prg: prg.to_vec(),
            chr: if chr_ram { alloc::vec![0u8; 0x2000] } else { chr.to_vec() },
            mirror,
            has_chr_ram: chr_ram,
        }
    }
}

impl MapperImpl for YourMapper {
    fn cpu_read(&mut self, addr: u16) -> u8 { /* ... */ 0 }
    fn cpu_write(&mut self, addr: u16, val: u8) { /* ... */ }
    fn ppu_read(&mut self, addr: u16) -> u8 { /* ... */ 0 }
    fn ppu_write(&mut self, addr: u16, val: u8) { /* ... */ }
    fn mirroring(&self) -> u8 { self.mirror }
    fn irq_pending(&self) -> bool { false }
    fn ack_irq(&mut self) {}
    fn has_chr_ram(&self) -> bool { self.has_chr_ram }
}
```
