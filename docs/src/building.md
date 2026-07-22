# Building & Testing

This chapter covers how to build, test, and profile x-nes across different targets.

## Quick Start

```bash
# Clone and build
git clone https://github.com/ChaoswareHQ/x-nes
cd x-nes

# Build the library
cargo build

# Run the headless example with a ROM
cargo run --example main -- "path/to/rom.nes"

# Run the graphical window example
cargo run --example window -- "path/to/rom.nes"

# Run all tests
cargo test
```

## Build Targets

### Desktop (default)

```bash
cargo build --release
```

Produces:
- `target/release/nes.dll` (Windows)
- `target/release/libnes.so` (Linux)
- `target/release/libnes.dylib` (macOS)
- `target/release/libnes.rlib` (Rust static lib)

### Shared library with FFI

```bash
cargo build --release --no-default-features --features ffi
```

This exposes C-compatible symbols (`nes_init`, `nes_load_rom`, etc.) for use from other languages.

### Static library for embedded

```bash
rustup target add thumbv7em-none-eabihf
cargo build --release --target thumbv7em-none-eabihf --no-default-features --features ffi
```

### WebAssembly (future)

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
```

## Test Suite

### Unit Tests

```bash
cargo test                          # All unit tests
cargo test --lib                    # Library tests only
cargo test rom                      # ROM parser tests only
```

The unit tests cover:
- **ROM parsing:** iNES 1.0/2.0 detection, mapper identification, mirroring, trainer, PRG/CHR sizing
- **Mapper creation:** NROM, UxROM, MMC3 initialization and basic reads
- **Bank switching:** MMC3 PRG bank switching, SMB3 header verification

### AccuracyCoin

```bash
cargo test --test accuracy_coin
```

This runs the comprehensive AccuracyCoin test suite (141 tests) that validates:
- CPU instructions (all official + unofficial)
- PPU timing (VBlank, NMI, sprite evaluation)
- APU channels and frame counter
- DMA (OAM DMA, DMC DMA)
- Controller input
- Edge cases (open bus, dummy reads, bus conflicts)

Current score: **86/141 pass, 55 fail** (61%)

### Blargg Tests (external)

Download Blargg's test ROMs and run them:

```bash
# CPU instruction test
cargo run --example main -- "tests/nestest.nes"

# PPU VBlank/NMI test
cargo run --example main -- "tests/blargg_ppu_tests/ppu_vbl_nmi.nes"
```

## Performance Profiling

### Release Build

Always test performance with `--release`. Debug builds are 10-100x slower:

```bash
cargo run --release --example main -- "rom.nes"
```

### Benchmark Frames Per Second

```rust
let start = std::time::Instant::now();
let target_frames = 600; // 10 seconds at 60fps
let mut frame_count = 0u32;

while frame_count < target_frames {
    tick(&mut cpu, &mut bus);
    if bus.ppu.frame_complete {
        bus.ppu.frame_complete = false;
        frame_count += 1;
    }
}

let elapsed = start.elapsed();
let fps = frame_count as f64 / elapsed.as_secs_f64();
println!("{:.1} FPS (target: 60.0)", fps);
```

### CPU Profiling

Use `perf` on Linux or `samply` (cross-platform):

```bash
# Linux
perf record --call-graph dwarf cargo run --release --example main -- "rom.nes"
perf report

# Cross-platform
cargo install samply
samply record cargo run --release --example main -- "rom.nes"
```

### Cycle Counting

Add a cycle counter to find expensive code paths:

```rust
use std::time::Instant;

let start = Instant::now();
for _ in 0..10_000_000 {
    tick(&mut cpu, &mut bus);
}
let elapsed = start.elapsed();
let ns_per_tick = elapsed.as_nanos() as f64 / 10_000_000.0;
println!("{} ns per tick ({} MHz emulated)", ns_per_tick, 1000.0 / ns_per_tick);
```

## Debug Builds

The debug profile (`cargo build`) is useful for:
- Faster compile times during development
- Debug symbols for breakpoints
- Assertions and overflow checks

But it's 10-100x slower than release. Use `--release` for performance testing.

### Dev Profile Optimizations

The `Cargo.toml` dev profile is configured for faster iteration:

```toml
[profile.dev]
panic = "abort"    # Smaller binaries, no unwinding overhead
```

## Crate Structure

```
x-nes/
├── Cargo.toml          # Library + example config
├── src/
│   ├── lib.rs          # Main tick/reset/NMI/IRQ functions
│   ├── cpu.rs          # CPU register struct (RP2A03)
│   ├── ops.rs          # Instruction implementations (256-entry jump table)
│   ├── bus.rs          # Memory bus, PPU sync, OAM DMA
│   ├── rom.rs          # iNES parser, mapper dispatch
│   ├── controller.rs   # Gamepad input
│   ├── clock.rs        # Timing conversions
│   ├── address.rs      # Memory region classification
│   ├── interrupt.rs    # Vector address constants
│   ├── apu/            # Audio Processing Unit
│   │   ├── mod.rs      # APU state machine, mixer
│   │   ├── pulse.rs    # Square wave channels
│   │   ├── triangle.rs # Triangle wave channel
│   │   ├── noise.rs    # Noise channel (LFSR)
│   │   └── dmc.rs      # Delta Modulation Channel
│   ├── ppu/            # Picture Processing Unit
│   │   ├── mod.rs      # PPU state, tick, timing
│   │   ├── registers.rs # PPU register I/O
│   │   ├── bus.rs       # VRAM address space, NT/palette access
│   │   └── render.rs    # Background/sprite pixel rendering
│   └── mapper/         # Cartridge mappers
│       ├── mod.rs       # MapperImpl trait, Mapper enum
│       ├── nrom.rs      # Mapper 0 (NROM)
│       ├── mmc1.rs      # Mapper 1 (MMC1)
│       ├── uxrom.rs     # Mapper 2 (UxROM)
│       ├── cnrom.rs     # Mapper 3 (CNROM)
│       ├── mmc3.rs      # Mapper 4 (MMC3)
│       ├── mmc5.rs      # Mapper 5 (MMC5)
│       ├── axrom.rs     # Mapper 7 (AxROM)
│       └── gxrom.rs     # Mapper 66 (GxROM)
├── examples/
│   ├── main.rs         # Headless emulator (console output only)
│   └── window.rs       # GUI emulator (softbuffer + winit, with audio)
├── docs/
│   └── src/            # This documentation (mdBook)
└── tests/              # Test ROMs and test harnesses
```

## Adding Features

### Adding a Feature Flag

In `Cargo.toml`:
```toml
[features]
my_feature = []
```

In code:
```rust
#[cfg(feature = "my_feature")]
fn extra_functionality() { ... }
```

Build with:
```bash
cargo build --features my_feature
```

### Adding a New Example

1. Create `examples/my_example.rs`
2. Add to `Cargo.toml`:
```toml
[[example]]
name = "my_example"
path = "examples/my_example.rs"
```
3. Run: `cargo run --example my_example`

## Continuous Integration

The project uses GitHub Actions (`.github/workflows/ci.yml`). CI runs:
- `cargo build` and `cargo test` on Linux, Windows, macOS
- `cargo clippy` for linting
- `cargo fmt --check` for formatting

### Running CI Locally

```bash
# Check formatting
cargo fmt --check

# Run linter
cargo clippy -- -D warnings

# Run all tests
cargo test --all
```

## Common Build Issues

### "file too big" when building for MCU

Reduce binary size:
```toml
[profile.release]
opt-level = "s"     # Optimize for size
lto = true
codegen-units = 1
strip = true
```

### "can't find crate for `std`" on embedded

Make sure you're using `--no-default-features` and targeting a `no_std` platform.

### Softbuffer/winit errors on Wayland

Set environment variable:
```bash
WINIT_UNIX_BACKEND=x11 cargo run --example window -- "rom.nes"
```

### "missing `config`" or other dependency errors

The window example needs dev-dependencies:
```bash
cargo run --example window   # NOT --lib or --release-only
```

These are in `[dev-dependencies]`, available when building examples and tests.
