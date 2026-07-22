# Architecture Overview

The NES is a carefully balanced system where three processors — CPU, PPU, and APU — run in lockstep. x-nes models this with a synchronous tick-based architecture.

## Data Flow

```
┌──────────────┐    opcode      ┌─────────┐
│     Bus       │◄─────────────►│   CPU   │
│               │               │         │
│  ┌──────────┐ │   address     │ 6502    │
│  │ RAM 2KB  │ │   data        │ core    │
│  ├──────────┤ │               └─────────┘
│  │ PPU regs │ │   cycles × 3      │
│  │ $2000-7  │ │◄─────────────────┘
│  ├──────────┤ │                    │
│  │ APU regs │ │   cycles           │
│  │ $4000-17 │ │◄─────────────────┘
│  ├──────────┤ │                    │
│  │ Mapper   │ │   tick() in lib.rs
│  │ (PRG ROM,│ │   orchestrates all
│  │  CHR ROM,│ │   three components
│  │  ExRAM)  │ │
│  └──────────┘ │
└──────────────┘
```

## The Tick Function

The heart of the emulator is in `lib.rs`. The current implementation is more sophisticated than early versions, handling penultimate-cycle NMI sampling, SH instruction DMC timing, and deferred NMI from $2000 writes:

```rust
pub fn tick(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let start_cycle = bus.cpu_cycle;
    let mut cycles_extra = 0u8;

    // 1. Fetch opcode from the address PC points to
    let opcode = bus.read(cpu.pc());
    cpu.set_pc(cpu.pc().wrapping_add(1));

    // 2. Set up penultimate-cycle NMI sampling point
    let base_cycles = BASE_CYCLES[opcode as usize] as u64;
    bus.penultimate_sample_cycle = start_cycle + base_cycles.saturating_sub(2);

    // 3. Handle CLI/SEI one-instruction IRQ latency
    let is_cli_sei = opcode == 0x58 || opcode == 0x78;
    let i_flag_for_irq = if is_cli_sei {
        cpu.get_flag(FLAG_INTERRUPT)
    } else { false };

    // 4. Pre-tick APU and handle SH instruction DMC saving
    bus.apu.tick(base_cycles as u16);

    // 5. Dispatch instruction — TABLE[opcode](cpu, bus)
    let cycles = TABLE[opcode as usize](cpu, bus) as u64;

    // 6. Sync PPU for remaining cycles (catch_up_ppu uses cpu_cycle)
    bus.cpu_cycle = start_cycle + cycles;
    bus.catch_up_ppu();

    // 7. Tick remaining APU cycles
    if cycles > base_cycles {
        bus.apu.tick_without_dmc((cycles - base_cycles) as u16);
    }

    // 8. Handle SH instruction DMC post-tick
    // ... (special handling for SHA/SHS/SHY/SHX opcodes)

    // 9. Check NMI (from VBlank or deferred $2000 write)
    if bus.ppu.nmi_from_vblank || bus.ppu.nmi_deferred_pending {
        bus.ppu.nmi_from_vblank = false;
        bus.ppu.nmi_deferred_pending = false;
        nmi(cpu, bus);
    }
    // 10. Check IRQ (with CLI/SEI latency)
    else if !cpu.get_flag(FLAG_INTERRUPT) && bus.poll_irq() {
        irq(cpu, bus);
    }

    (cycles + cycles_extra as u64) as u8
}
```

## PPU Synchronization

The PPU runs at 3× the CPU speed. x-nes uses a **catch-up** model:

- `bus.cpu_cycle` tracks the current CPU cycle globally
- `bus.ppu_sync_cycle` tracks the last cycle the PPU was synced to
- When the bus accesses PPU registers or the CPU finishes an instruction, `catch_up_ppu()` advances the PPU by `(cpu_cycle - ppu_sync_cycle) * 3` dots

```rust
pub fn catch_up_ppu(&mut self) {
    if self.cpu_cycle > self.ppu_sync_cycle {
        let ppu_dots = (self.cpu_cycle - self.ppu_sync_cycle) * 3;
        self.ppu.tick_batch(ppu_dots as u16, &mut self.mapper);
        self.ppu_sync_cycle = self.cpu_cycle;
    }
}
```

This is called on every bus read/write so that PPU register access always sees up-to-date PPU state.

## NES Timing

The NES runs on a ~21.47727 MHz master clock, divided into:

| Component | Divisor | Frequency | Ratio per CPU cycle |
|-----------|---------|-----------|-------------------|
| CPU | /12 | ~1.79 MHz | 1 |
| PPU | /4 | ~5.37 MHz | 3 |
| APU | /12 | ~1.79 MHz | 1 |

So for every CPU cycle, the PPU advances 3 dots. The APU runs at roughly the same rate as the CPU.

## Module Responsibilities

| Module | Role |
|--------|------|
| `cpu.rs` | Register file, flag manipulation, PC management |
| `ops.rs` | 256-entry jump table with all instruction implementations |
| `bus.rs` | Memory routing, PPU catch-up sync, OAM DMA, open bus emulation |
| `ppu/mod.rs` | PPU state machine, scanline timing, NMI edge detection |
| `ppu/render.rs` | On-the-fly background and sprite pixel rendering |
| `ppu/registers.rs` | PPU register read/write ($2000-$2007) |
| `ppu/bus.rs` | VRAM address space, NT mapping, palette access, open bus decay |
| `apu/mod.rs` | APU state, frame sequencer, mixer, register I/O |
| `apu/pulse.rs` | Square wave channels (pulse 1, pulse 2) |
| `apu/triangle.rs` | Triangle wave channel |
| `apu/noise.rs` | Noise channel (LFSR) |
| `apu/dmc.rs` | Delta Modulation Channel (DMC) |
| `rom.rs` | iNES 1.0/2.0 ROM parser, mapper dispatch |
| `mapper/mod.rs` | Mapper trait, Mapper enum, dispatch methods |
| `mapper/*.rs` | Individual mapper implementations (8 mappers) |
| `controller.rs` | Gamepad input (standard NES controller) |
| `clock.rs` | Master/CPU/PPU cycle conversion |
| `address.rs` | Memory region classification helpers |
| `interrupt.rs` | Vector address constants (NMI=$FFFA, RESET=$FFFC, IRQ=$FFFE) |

## Open Bus Emulation

x-nes emulates the NES open bus behavior — when reading unmapped addresses, the last value on the data bus is returned (not 0). The bus also simulates capacitive decay:

```rust
pub fn get_open_bus(&self) -> u8 {
    const DECAY_CYCLES: u64 = 5_000_000; // ~1 second
    let elapsed = self.tick_count.saturating_sub(self.last_bus_write_tick);
    if elapsed >= DECAY_CYCLES { return 0; }
    let decay = (elapsed * 255 / DECAY_CYCLES) as u8;
    self.last_bus_value & !(decay | decay >> 1 | decay >> 2)
}
```

## NMI Timing (Penultimate Cycle Rule)

The real NES samples NMI on the **penultimate cycle** (second-to-last) of each instruction. This means if a write to $2000 enables NMI on the last cycle of an instruction, the NMI is deferred to the NEXT instruction. x-nes implements this via:

```rust
bus.penultimate_sample_cycle = start_cycle + base_cycles.saturating_sub(2);
```

On each bus access, if the current cycle matches the penultimate cycle and NMI is latched, the NMI is deferred:

```rust
fn sample_penultimate(&mut self) {
    if self.cpu_cycle > 0
        && (self.cpu_cycle - 1) == self.penultimate_sample_cycle
        && self.ppu.nmi_latched
    {
        self.ppu.nmi_latched = false;
        self.ppu.nmi_deferred_pending = true;
    }
}
```

## Mapper Architecture

Mappers use a trait-based dispatch system. See the [Mappers Overview](mappers.md) for full details.

## no_std Design

The emulator uses `#![cfg_attr(not(any(test, feature = "std")), no_std)]`. When `std` is enabled (tests, examples), the emulator can use `Vec` and heap allocation. On `no_std` targets, all state is stack-allocated or uses `alloc` for Vec-based PRG/CHR storage.
