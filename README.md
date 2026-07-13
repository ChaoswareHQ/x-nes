# x-nes

A minimal NES emulator written in Rust.

## Features

- 6502 CPU emulation
- No standard library dependency
- Targets both desktop (cdylib) and embedded (staticlib)

## Building

```sh
# Desktop
cargo build --release

# Microcontroller (Cortex-M4 example)
cargo build --release --target thumbv7em-none-eabihf
```

## Project Structure

| Module | Description |
|--------|-------------|
| `cpu` | 6502 CPU registers and execution |
| `ppu` | Picture Processing Unit |
| `apu` | Audio Processing Unit |
| `bus` | System bus and memory mapping |
| `rom` | ROM loading and parsing |
| `ops` | CPU instruction set |

## Usage

Link the library into your host application:

```c
// C host example
#include <stdint.h>

extern void nes_init(uint16_t reset_vector);
extern void nes_tick(void);

int main(void) {
    nes_init(0x8000);
    while (1) {
        nes_tick();
    }
    return 0;
}
```

## License

MIT
