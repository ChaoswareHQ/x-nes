# x-nes

A lightweight NES emulator core written in Rust, designed to run everywhere – from microcontrollers to modern desktops. It exposes a clean Rust API and a C‑compatible FFI for use from Lua, C, or any other language.

**[Read the Book](https://chaoswarehq.github.io/x-nes/)** — a complete walkthrough of the architecture, instruction set, PPU, and performance techniques.

## Features

- **Cycle‑accurate 6502 CPU** – all 56 instructions, 13 addressing modes
- **PPU** – scanline‑accurate rendering with NMI generation
- **iNES ROM parser** – mapper and mirroring metadata support
- **`no_std` by default** – works on bare metal (MCUs, MPUs)
- **Dual library output** – both `cdylib` (shared) and `staticlib` for flexible integration
- **C‑compatible FFI** – exposes `nes_*` functions for easy embedding (optional, via `ffi` feature)
- **Optional features** – `save_states`, `rewind`, and `std` for testing
- **Tiny footprint** – ~150‑200 KB shared library (fully stripped)

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
let mut bus = Bus::new(&rom.prg, &rom.chr, rom.mirroring);
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
| `ops` | 256‑entry jump table with all instruction implementations |
| `bus` | Memory bus with RAM, PPU/APU routing, and OAM DMA |
| `ppu` | Picture Processing Unit with scanline timing and sprite evaluation |
| `apu` | Audio Processing Unit (pulse channels, sample buffer) |
| `rom` | iNES ROM header parser and NROM mapper support |
| `ffi` | Optional C‑compatible API (enabled by `ffi` feature) |
| `clock` | Master clock cycle conversions |
| `interrupt` | Vector address constants (NMI, RESET, IRQ) |
| `address` | Memory region classification helpers |

## Book

The [x-nes book](https://chaoswarehq.github.io/x-nes/) dives deep into the emulator:

1. The RP2A03 CPU – registers, flags, memory model
2. Instruction Set & Addressing Modes – all 13 modes with timing
3. Memory & I/O – bus dispatch, mirroring, PPU/APU routing
4. Picture Processing Unit – scanlines, registers, NMI generation
5. Audio & ROM Loading – APU channels, iNES format
6. Performance & Portability – `no_std` design, cross‑compilation, optimizations

```sh
# Build the book locally
cargo install mdbook
mdbook serve docs/
```

## License

MIT OR Apache-2.0