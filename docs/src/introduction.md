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
- **Practical** — it currently supports CPU execution, bus routing, PPU timing, ROM loading, and NROM mapper behavior

The current implementation focuses on the core emulator loop and the main NES subsystems. Audio support is still intentionally lightweight, with the APU exposing register handling and timing hooks rather than full sound synthesis.

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
