# x-nes

A cycle-accurate NES emulator written in Rust, targeting everything from microcontrollers to modern desktops.

## What is an Emulator?

An emulator is a program that mimics the behavior of a hardware system. For the NES, this means simulating:

- **The CPU** (6502) — executes game code
- **The PPU** — generates the video signal
- **The APU** — generates the audio signal
- **The Memory Bus** — routes data between components and the cartridge

All running in lockstep, synchronized to the NES's master clock.

## What is x-nes?

x-nes is a from-scratch implementation of the NES hardware in Rust. It is:

- **Cycle-accurate** — the CPU follows the expected cycle timing for each instruction
- **no_std** — no operating system dependencies, so it can run on bare metal
- **Portable** — the same core code is shared across desktop and embedded targets
- **Practical** — CPU execution, bus routing, PPU timing, ROM loading, and multiple mapper implementations

The current implementation passes **86% of the AccuracyCoin test suite**, with all Blargg CPU and PPU tests passing cleanly. Audio support is functional but still needs refinement — timing is slightly fast and some tones are off.

## Want to Contribute?

x-nes is an open-source project aiming to be the most accurate NES emulator library in Rust. We're actively looking for help with:

- **MMC3 mapper** — needed for games like Super Mario Bros. 3 and Mega Man 3
- **Audio accuracy** — APU timing and tone refinement
- **PPU edge cases** — sprite evaluation corner cases
- **RetroArch integration** — libretro core support
- **New mappers** — implement additional mappers as your favorite games require

[Contributing guide](https://github.com/ChaoswareHQ/x-nes) | [Issue tracker](https://github.com/ChaoswareHQ/x-nes/issues)

## How This Book is Organized

| Chapter | What You'll Learn |
|---------|-------------------|
| 1. The 6502 CPU | CPU registers, flags, memory model |
| 2. Instruction Set | All 56 instructions, 13 addressing modes |
| 3. Memory & I/O | NES memory map, PPU/APU registers |
| 4. PPU | Video generation, scanlines, NMI |
| 5. Audio & ROM | APU channels, iNES format |
| 6. Performance | Cross-platform design, optimization |

Each chapter explains the hardware first, then shows the Rust implementation. The code is designed to match the hardware behavior as closely as possible.
