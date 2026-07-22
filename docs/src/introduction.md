# x-nes

A cycle-accurate NES emulator written in Rust, targeting everything from microcontrollers to modern desktops.

## What is an Emulator?

An emulator is a program that mimics the behavior of a hardware system. For the NES, this means simulating:

- **The CPU** (6502) — executes game code, handles interrupts
- **The PPU** — generates the video signal, scanlines, sprites, and scrolling
- **The APU** — generates the audio signal (5 channels: 2 pulse, triangle, noise, DMC)
- **The Memory Bus** — routes data between components and the cartridge mapper
- **The Mapper** — cartridge chip that provides bank switching, IRQs, extra RAM, and custom features

All running in lockstep, synchronized to the NES's master clock.

## What is x-nes?

x-nes is a from-scratch implementation of the NES hardware in Rust. It is:

- **Cycle-accurate** — the CPU follows the expected cycle timing for each instruction, including page-cross penalties
- **no_std** — no operating system dependencies, so it can run on bare metal
- **Portable** — the same core code is shared across desktop and embedded targets
- **Practical** — 8 mappers implemented (NROM, MMC1, UxROM, CNROM, MMC3, MMC5, AxROM, GxROM), covering ~90% of the NES library
- **Full-featured APU** — all 5 audio channels with frame sequencer, sweep units, envelope generators, and DMC DMA

The current implementation passes **86% of the AccuracyCoin test suite** (86/141 tests), with all Blargg CPU tests passing cleanly.

## Current Status

| Component | Status |
|-----------|--------|
| CPU (all 151 opcodes) | ✅ Cycle-accurate, all unofficial opcodes |
| PPU rendering | ✅ On-the-fly renderer with NMI edge detection |
| PPU scrolling | ✅ $2005/$2006 with loopy-V register, fine X/Y |
| APU (5 channels) | ⚠️ Functional, timing needs refinement |
| Mapper 0 (NROM) | ✅ Complete |
| Mapper 1 (MMC1) | ✅ Complete |
| Mapper 2 (UxROM) | ✅ Complete |
| Mapper 3 (CNROM) | ✅ Complete |
| Mapper 4 (MMC3) | ✅ Complete — SMB3, Mega Man 3, Kirby support |
| Mapper 5 (MMC5) | ✅ Complete — Castlevania III support |
| Mapper 7 (AxROM) | ✅ Complete |
| Mapper 66 (GxROM) | ✅ Complete |
| Open bus emulation | ✅ With capacitive decay |
| NMI penultimate cycle | ✅ Deferred NMI from $2000 writes |
| iNES 2.0 headers | ✅ Correct detection |
| AccuracyCoin | ⚠️ 86/141 (61%) |
| Expansion audio (MMC5) | ❌ Not yet implemented |

## Highlights

### MMC5 Support

x-nes is one of the few emulators with working MMC5 support, enabling Castlevania III and other rare MMC5 games:

- 4 PRG modes (32KB, 2×16KB, 16KB+8KB+8KB, 4×8KB) with up to 1MB PRG ROM
- 4 CHR modes (8KB, 4KB, 2KB, 1KB) with separate BG/sprite banking and up to 1MB CHR ROM
- ExRAM with 4 operating modes (extra nametable, extended attributes, CPU RAM, read-only)
- Scanline IRQ for split-screen effects
- Hardware 8×8→16 multiplier at $5205/$5206
- Fill mode and per-nametable source mapping via $5105

### On-the-Fly Rendering

Instead of emulating the PPU's internal shift registers (which would require complex tile caching), x-nes computes each pixel directly from scroll state snapshots. This approach:
- Simplifies the renderer significantly
- Naturally supports MMC5 extended attributes (per-tile CHR bank + palette)
- Uses `render_v` snapshots at prerender cycle 0 and cycle 257

## Want to Contribute?

x-nes is an open-source project aiming to be the most accurate NES emulator library in Rust. We're actively looking for help with:

- **Audio accuracy** — APU timing and tone refinement
- **PPU edge cases** — sprite evaluation corner cases, OAM corruption
- **DMA timing** — DMC DMA + OAM DMA interaction
- **Expansion audio** — MMC5 extra pulse channels, VRC6, etc.
- **New mappers** — VRC series, FME-7, Namco 163, etc.
- **RetroArch integration** — libretro core support

[Contributing guide](https://github.com/ChaoswareHQ/x-nes) | [Issue tracker](https://github.com/ChaoswareHQ/x-nes/issues)

## How This Book is Organized

| Chapter | What You'll Learn |
|---------|-------------------|
| 1. Architecture | System design, tick function, PPU sync, open bus |
| 2. The 6502 CPU | CPU registers, flags, stack, memory model |
| 3. Instruction Set | All 56 official + unofficial opcodes, 13 addressing modes |
| 4. Memory & I/O | NES memory map, PPU/APU registers, OAM DMA |
| 5. PPU | Video generation, scanlines, scrolling, NMI, on-the-fly rendering |
| 6. Audio & ROM | APU channels, frame sequencer, iNES format |
| 7. Mappers | All 8 mappers, bank switching, IRQs, CHR banking |
| 8. MMC5 Deep Dive | Complete register map, ExRAM modes, extended attributes |
| 9. Debugging | CPU/PPU/mapper debugging, crash triage, diagnostic tools |
| 10. Graphics Debugging | Visual symptom diagnosis, pixel inspection, frame comparison |
| 11. Performance | Cross-platform design, optimization techniques |
| 12. Building & Testing | Build targets, test suite, profiling, CI |

Each chapter explains the hardware first, then shows the Rust implementation. The code is designed to match the hardware behavior as closely as possible.
