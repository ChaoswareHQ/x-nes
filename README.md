# x-nes

[![Crates.io Version](https://img.shields.io/crates/v/x-nes)](https://crates.io/crates/x-nes)
[![Crates.io Downloads](https://img.shields.io/crates/d/x-nes)](https://crates.io/crates/x-nes)
[![docs.rs](https://img.shields.io/docsrs/x-nes)](https://docs.rs/x-nes)
[![CI](https://img.shields.io/github/actions/workflow/status/ChaoswareHQ/x-nes/ci.yml?branch=main)](https://github.com/ChaoswareHQ/x-nes/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](#license)
[![Rust](https://img.shields.io/badge/rust-1.97%2B-orange)](https://www.rust-lang.org)

A lightweight, cycle-accurate NES emulator **library** written in Rust, designed to run everywhere — from microcontrollers to modern desktops. Exposes a clean Rust API and a C-compatible FFI for use from Lua, C, or any other language.

**[Read the Book](https://chaoswarehq.github.io/x-nes/)** — a complete walkthrough of the architecture, instruction set, PPU, and performance techniques.

---

## Status

> **Beta.** The core emulation is solid for many games, but there are known issues:
>
> - **Audio** — timing is slightly fast, some tones are off. APU is functional but needs refinement.
> - **Graphics** — some games render incorrectly; edge-case PPU behaviors are still being ironed out.
> - **MMC3 mapper** (used by Super Mario Bros. 3, Mega Man 3, etc.) — **not yet supported.** Currently implements NROM, UxROM, CNROM, AxROM, GxROM, and MMC1.
> - **Accuracy** — passes **61% of the AccuracyCoin test suite** (86/141 pass). Blargg tests: **20/21 pass** (ppu_vbl_nmi fails on NMI timing subtest 5).
>
> **AccuracyCoin failures by area:**
>
> | Area | Fails | Key failures |
> |------|-------|-------------|
> | APU / Audio | 6 | Frame counter IRQ, DMC channel, length counter timing |
> | DMA / Bus | 7 | DMC+OAM DMA, bus conflicts, DMA + open bus |
> | NMI / Interrupts | 6 | NMI+BRK, NMI+IRQ, I-flag latency, VBlank timing |
> | PPU edge cases | 15 | Palette RAM quirks, sprite 0/scaling, OAM corruption, stale shift regs, rendering flags, $2007 stress |
> | CPU / Addressing | 8 | Implied dummy reads, branch dummy reads, addressing mode edge cases |
> | Controller | 2 | Strobing, clocking |
> | Other | 11 | Internal data bus, ALE+read, hybrid addresses, JSR edge cases, $2004 stress |
>
> **Total: 86 pass, 55 fail, 0 skip**

See the [issues page](https://github.com/ChaoswareHQ/x-nes/issues) for the full roadmap.

---

## Features

- **Cycle-accurate 6502 CPU** — all 56 instructions, 13 addressing modes
- **PPU** — scanline-accurate rendering with NMI generation
- **iNES ROM parser** — mapper and mirroring metadata support
- **`no_std` by default** — works on bare metal (MCUs, MPUs)
- **Dual library output** — `lib`, `cdylib` (shared), and `staticlib` for flexible integration
- **C-compatible FFI** — exposes `nes_*` functions for easy embedding (optional, via `ffi` feature)
- **Optional features** — `save_states`, `rewind`, `retroarch`, and `std` for testing (in development)
- **Tiny footprint** — ~150–200 KB shared library (fully stripped)

## Building

```sh
# Desktop shared library (.dll / .so / .dylib)
cargo build --release --no-default-features --features ffi

# Static library for MCU (ARM Cortex-M4 example)
rustup target add thumbv7em-none-eabihf
cargo build --target thumbv7em-none-eabihf --release --no-default-features --features ffi

# Example with GUI (requires std)
cargo run --release --example window -- "your-rom.nes"
```

## Usage

### As a Rust crate (via crates.io)

Add to your `Cargo.toml`:

```toml
[dependencies]
x-nes = "0.1.0"
```

Enable the `ffi` feature if you need C exports:

```toml
x-nes = { version = "0.1.0", features = ["ffi"] }
```

### Basic emulation loop (Rust)

```rust
use nes::{bus::Bus, cpu::CpuRp2a03, rom::Rom};
use nes::{reset, tick};

let data = std::fs::read("game.nes").unwrap();
let rom = Rom::new(&data).unwrap();

let mut cpu = CpuRp2a03::new(0x0000);
let mut bus = Bus::new(rom.create_mapper());
reset(&mut cpu, &mut bus);

loop {
    tick(&mut cpu, &mut bus);
    if bus.ppu.frame_complete {
        // Frame ready – pixels in bus.ppu.frame (256×240)
        bus.ppu.frame_complete = false;
    }
}
```

### Using the C FFI (from Lua, C, etc.)

The FFI exports functions with the `nes_` prefix. Example with LuaJIT:

```lua
local ffi = require("ffi")
local lib = ffi.load("nes")   -- or "libnes" on Linux

ffi.cdef[[
    int nes_init(void);
    int nes_load_rom(const uint8_t* data, size_t len);
    void nes_reset(void);
    uint8_t nes_step(void);
    void nes_run_frame(void);
    uint8_t nes_read_cpu(uint16_t addr);
    void nes_write_cpu(uint16_t addr, uint8_t val);
    const uint8_t* nes_get_frame_ptr(void);
    // ... see src/ffi.rs for full list
]]

lib.nes_init()
local rom = io.open("game.nes", "rb"):read("*all")
lib.nes_load_rom(rom, #rom)
lib.nes_reset()

while true do
    lib.nes_run_frame()
    local frame = lib.nes_get_frame_ptr()
    -- frame points to 256*240 bytes of palette indices
end
```

## Project Structure

| Module | Description |
|--------|-------------|
| `cpu` | RP2A03 CPU registers, flags, and instruction fetch |
| `ops` | 256-entry jump table with all instruction implementations |
| `bus` | Memory bus with RAM, PPU/APU routing, and OAM DMA |
| `ppu` | Picture Processing Unit with scanline timing and sprite evaluation |
| `apu` | Audio Processing Unit (pulse channels, sample buffer) |
| `rom` | iNES ROM header parser and mapper dispatch |
| `mapper` | Mapper implementations (NROM, UxROM, CNROM, AxROM, GxROM, MMC1) |
| `ffi` | Optional C-compatible API (enabled by `ffi` feature) |
| `clock` | Master clock cycle conversions |
| `interrupt` | Vector address constants (NMI, RESET, IRQ) |
| `address` | Memory region classification helpers |

## Roadmap

- [x] Cycle-accurate CPU + official instructions
- [x] PPU scanline rendering + NMI
- [x] APU (basic timing, pulse channels)
- [x] iNES ROM parser + NROM, UxROM, CNROM, AxROM, GxROM, MMC1
- [ ] MMC3 mapper (SMB3, Mega Man 3, etc.)
- [ ] Audio accuracy refinements
- [ ] PPU edge-case fixes (sprite evaluation corner cases)
- [ ] RetroArch integration
- [ ] WASM target support

## Contributing

Contributions are welcome! This project aims to be the most accurate NES emulator library in Rust. If you can fix a bug, implement a mapper, or improve audio timing, jump in.

- Open [issues](https://github.com/ChaoswareHQ/x-nes/issues) for bugs or feature requests
- PRs are reviewed promptly
- See the book for architecture docs before diving in

## Support

If you find this project useful, consider supporting its development:

[![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-azzodude-yellow?logo=buymeacoffee)](https://buymeacoffee.com/azzodude)

Your support helps sustain ongoing work on accuracy, mappers, and features like RetroArch integration.

## License

MIT OR Apache-2.0
