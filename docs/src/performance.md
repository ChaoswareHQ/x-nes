# 6. Performance & Portability

## 6.1 Design for no_std

x-nes is built without the Rust standard library. This means it can run on any platform with a Rust compiler, from 8-bit microcontrollers to 64-bit desktops.

```rust
#![cfg_attr(not(test), no_std)]
```

The `cfg_attr` allows `std` during testing for convenience while keeping the release build minimal.

### 6.1.1 No Heap Allocation

The emulator never allocates memory. ROM data is borrowed from the caller as `&[u8]`, and all internal state is fixed-size arrays:

| State | Size | Type |
|-------|------|------|
| CPU registers | 7 bytes | `[u8; 7]` |
| CPU RAM | 2KB | `[u8; 2048]` |
| PPU VRAM | 4KB | `[u8; 0x1000]` |
| PPU pattern tables | 8KB | `[u8; 0x2000]` |
| PPU palette | 32 bytes | `[u8; 0x20]` |
| PPU OAM | 256 bytes | `[u8; 0x100]` |
| PPU frame buffer | 61KB | `[u8; 256 * 240]` |
| **Total** | **~75KB** | |

### 6.1.2 Custom Panic Handler

Without `std`, there's no default panic handler. x-nes provides its own:

```rust
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
```

On panic, the CPU loops forever. A debugger can breakpoint on this function to inspect state.

## 6.2 Cross-Platform Configuration

### 6.2.1 Cargo Configuration (`.cargo/config.toml`)

```toml
[target.'cfg(any(target_arch = "x86_64", target_arch = "aarch64"))']
rustflags = ["-C", "target-cpu=native"]

[target.thumbv7em-none-eabihf]
rustflags = ["-C", "opt-level=s"]
```

Desktop builds use `target-cpu=native` to let LLVM use every instruction your CPU supports. MCU builds optimize for size.

### 6.2.2 Rust Toolchain (`rust-toolchain.toml`)

```toml
[toolchain]
channel = "stable"
targets = [
    "x86_64-pc-windows-msvc",
    "thumbv7em-none-eabihf",
]
```

### 6.2.3 Cargo.toml

```toml
[lib]
name = "nes"
crate-type = ["cdylib", "staticlib"]
```

`cdylib` produces a shared library (.dll/.so/.dylib) for desktop use. `staticlib` produces a static archive (.a) for embedded use.

## 6.3 Crate Type: cdylib vs staticlib

### cdylib (Dynamic Library)

Used when x-nes is loaded by another program at runtime:

```
Desktop app (C, Python, etc.)
          │
          │  FFI call
          ▼
    x-nes.dll / x-nes.so  ← built as cdylib
```

The host program calls `tick()`, `reset()`, etc. through a C-compatible interface.

### staticlib (Static Library)

Used when x-nes is linked directly into another program:

```
Firmware binary
    ├── game code
    ├── x-nes code  ← built as staticlib
    └── board support
```

Everything is linked into one binary — no runtime loading needed.

## 6.4 Release Profile

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
| `opt-level = 3` | Maximum optimization — prioritizes speed |
| `lto = true` | Link-time optimization across all dependency boundaries |
| `codegen-units = 1` | Compiler sees entire crate at once for better inlining |
| `strip = "symbols"` | Removes debug symbols from output |
| `panic = "abort"` | No unwind tables (required for no_std + cdylib) |

## 6.5 Build Targets

### Building for Desktop

```bash
cargo build --release
```

Output: `target/release/nes.dll` (Windows), `target/release/libnes.so` (Linux), or `target/release/libnes.dylib` (macOS)

### Building for Microcontroller (ARM Cortex-M)

```bash
rustup target add thumbv7em-none-eabihf
cargo build --release --target thumbv7em-none-eabihf
```

Output: `target/thumbv7em-none-eabihf/release/libnes.a`

## 6.6 Performance Design Decisions

### 6.6.1 7-Byte CPU Struct

The CPU state is exactly 7 bytes — it fits in a single 64-bit register on x86_64. Register accessors inline to 1-2 `mov` instructions.

### 6.6.2 Branchless Flag Operations

```
set_flag(flag, true):    sr = (sr & !flag) | (1 * flag) = sr | flag
set_flag(flag, false):   sr = (sr & !flag) | (0 * flag) = sr & !flag
```

The multiply by 0 or 1 replaces a branch. No branch mispredictions.

### 6.6.3 Jump Table Dispatch

`TABLE[opcode as usize](cpu, bus)` — a single indexed indirect call. No binary search, no match tree, no bounds check.

### 6.6.4 Bus Dispatch

Shifting the address right by 12 extracts the top nibble (0-15), which selects the memory region. A match on a small integer compiles to a compact jump table.

### 6.6.5 Page-Cross Penalty

`(base ^ addr) >> 8` is 0 if same page, 1 if different. Added directly to the cycle count — no branch needed.

## 6.7 Binary Sizes

| Build Configuration | Size |
|--------------------|------|
| Debug | ~50 KB |
| Release (x86_64) | ~9 KB |
| Release (ARM MCU) | ~4 KB |

The small size comes from:
1. No libstd (hundreds of KB of I/O, allocators, threading)
2. No unwind tables (panic = abort)
3. Dead code elimination (LTO + codegen-units=1)
4. Symbol stripping

## Summary

- x-nes is `no_std` — runs anywhere Rust runs
- No heap allocation — all state is fixed-size arrays
- Builds as both dynamic and static library
- Release profile strips everything unnecessary
- Same code compiles for desktop and microcontroller targets
- Binary sizes range from 4KB (MCU) to 9KB (desktop)
