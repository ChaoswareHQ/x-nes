# Portability: From Microcontrollers to x86_64

x-nes is designed to run on any platform with a Rust compiler — from 8-bit microcontrollers to 64-bit desktops.

## no_std

The entire emulator uses `#![no_std]`, which means:

- No dependency on the Rust standard library
- No OS syscalls (no file I/O, no threads, no networking)
- No heap allocator required
- No floating point (the 6502 has no FPU)

```rust
#![cfg_attr(not(test), no_std)]
```

The `cfg_attr(not(test))` allows `std` during testing so you can use `#[test]` and `cargo test`.

## Borrowed ROM Data

Instead of owning ROM data with `Vec<u8>`, the bus borrows it:

```rust
pub struct Bus<'a> {
    pub prg: &'a [u8],  // Borrowed, not owned
    pub ppu: Ppu,        // Owned, fixed-size
    pub apu: Apu,        // Owned, fixed-size
}
```

This means the caller — whether it's a desktop app, a webassembly runtime, or a microcontroller firmware — retains ownership of the ROM data. The emulator never allocates.

## Cross-Compilation Targets

### Desktop (cdylib)

```toml
[lib]
crate-type = ["cdylib", "staticlib"]
```

Produces a shared library (`.dll`/`.so`/`.dylib`) that can be linked from C, Python, or any language with FFI.

```bash
cargo build --release
```

### Microcontroller (staticlib)

```bash
# Add the target
rustup target add thumbv7em-none-eabihf

# Build for Cortex-M4
cargo build --release --target thumbv7em-none-eabihf
```

The result is a `.a` file (static library) that links into your firmware.

## Configuration Files

### `.cargo/config.toml`

```toml
[target.'cfg(any(target_arch = "x86_64", target_arch = "aarch64"))']
rustflags = ["-C", "target-cpu=native"]

[target.thumbv7em-none-eabihf]
rustflags = ["-C", "opt-level=s"]
```

Desktop builds use `target-cpu=native` for maximum instruction-level optimization. MCU builds optimize for size (`opt-level=s`).

### `rust-toolchain.toml`

```toml
[toolchain]
channel = "stable"
targets = [
    "x86_64-pc-windows-msvc",
    "thumbv7em-none-eabihf",
]
```

Auto-installs targets when cloning the repo.

## Release Profile

```toml
[profile.release]
opt-level      = 3
lto            = true
codegen-units  = 1
strip          = "symbols"
panic          = "abort"
```

| Setting | Effect |
|---------|--------|
| `lto = true` | Link-time optimization across all dependencies |
| `codegen-units = 1` | Single compilation unit — maximum optimization |
| `strip = "symbols"` | Removes debug symbols from output |
| `panic = "abort"` | No unwind tables (required for `no_std` + `cdylib`) |

## Test Harness (Desktop Only)

The `dev-dependencies` section in `Cargo.toml` includes desktop-only crates for testing:

```toml
[dev-dependencies]
softbuffer = "0.4.8"   # Software framebuffer
winit = "0.30.13"      # Window creation
config = "0.15.25"     # Configuration
```

These are only available when `std` is linked (i.e., during `cargo test` or when building tests).

## Panic Handler

Since there's no `std` to handle panics, a custom panic handler is required:

```rust
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
```

This loops forever on panic. For desktop debugging, you can breakpoint on this function.

## No Floating Point

The 6502 has no floating-point instructions. The emulator uses only integer arithmetic. This means:

- No `f32` or `f64` types
- No floating-point emulation
- Works on microcontrollers without FPUs
