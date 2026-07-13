# x-nes

A minimal NES emulator written in Rust, targeting everything from microcontrollers to modern desktops.

**[Read the Book](https://chaoswarehq.github.io/x-nes/)** — a complete walkthrough of the architecture, instruction set, PPU, and performance techniques.

## Features

- Cycle-accurate 6502 CPU emulation (56 instructions, 13 addressing modes)
- PPU with scanline-accurate timing and NMI generation
- iNES ROM parser (mapper 0 / NROM)
- No standard library dependency — works on bare metal
- Builds as both shared library (`.dll`/`.so`) and static library (`.a`)
- 9KB release binary for desktop, ~4KB for microcontroller targets

## Building

```sh
# Desktop (cdylib)
cargo build --release

# Microcontroller (staticlib, ARM Cortex-M4 example)
cargo build --release --target thumbv7em-none-eabihf

# Example (included input and gui)
cargo run --release --example window -- "your-rom.nes"
```

## Project Structure

| Module | Description |
|--------|-------------|
| `cpu` | 6502 CPU registers and branchless flag operations |
| `ops` | 256-entry jump table and all instruction implementations |
| `bus` | Memory bus with PPU/APU routing and OAM DMA |
| `ppu` | Picture Processing Unit with scanline timing |
| `apu` | Audio Processing Unit register stubs |
| `rom` | iNES ROM header parser and NROM mapper |
| `clock` | Master clock cycle conversions |
| `interrupt` | Vector address constants |
| `address` | Memory region classification helpers |

## Book

The [x-nes book](https://chaoswarehq.github.io/x-nes/) explains the emulator in detail:

1. The 6502 CPU — registers, flags, memory model
2. Instruction Set & Addressing Modes — all 13 modes, instruction categories
3. Memory & I/O — bus dispatch, RAM mirroring, PPU/APU routing
4. Picture Processing Unit — scanlines, registers, NMI generation
5. Audio & ROM Loading — APU channels, iNES format
6. Performance & Portability — no_std design, cross-compilation, optimizations

```sh
# Build the book locally
cargo install mdbook
mdbook serve docs/
```

## Usage

```rust
use nes::{rom::Rom, bus::Bus, cpu::CpuRp2a03};
use nes::{tick, reset};

let data = std::fs::read("game.nes").unwrap();
let rom = Rom::new(&data).unwrap();

let mut cpu = CpuRp2a03::new(0);
let mut bus = Bus::new(&rom.prg, &rom.chr);
reset(&mut cpu, &mut bus);

loop {
    tick(&mut cpu, &mut bus);
    if bus.ppu.frame_complete {
        // frame ready in bus.ppu.frame (256x240 pixels)
        bus.ppu.frame_complete = false;
    }
}
```

## License

MIT
