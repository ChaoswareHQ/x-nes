# Architecture Overview

The NES is a carefully balanced system where three processors — CPU, PPU, and APU — run in lockstep. x-nes models this with a synchronous tick-based architecture.

## Data Flow

```
┌──────────┐    opcode    ┌─────────┐
│   Bus    │◄───────────►│   CPU   │
│          │             │         │
│  ┌──────┐│  address    │ 6502    │
│  │ RAM  ││  data       │ core    │
│  ├──────┤│             └─────────┘
│  │ PPU  ││  cycles * 3     │
│  │ regs ││◄──────────────┘
│  ├──────┤│                 │
│  │ APU  ││  cycles         │
│  │ regs ││◄──────────────┘
│  ├──────┤│                 │
│  │ PRG  ││  tick() in lib.rs orchestrates
│  │ ROM  ││  all three components
│  └──────┘│
└──────────┘
```

## The Tick Function

The heart of the emulator is in `lib.rs`:

```rust
pub fn tick(cpu: &mut Cpu6502, bus: &mut Bus<'_>) -> u8 {
    // 1. Fetch opcode from the address CPU's PC points to
    let opcode = bus.read(cpu.pc());

    // 2. Advance program counter past the opcode byte
    cpu.set_pc(cpu.pc().wrapping_add(1));

    // 3. Dispatch to the correct instruction handler
    //    The instruction reads operands, executes, and returns cycle count
    let cycles = TABLE[opcode as usize](cpu, bus);

    // 4. Advance PPU by 3 cycles per CPU cycle (NES timing ratio)
    bus.ppu.tick_batch((cycles as u16) * 3);

    // 5. Advance APU by CPU cycles
    bus.apu.tick(cycles);

    // 6. Check if PPU triggered an NMI during its cycles
    if bus.poll_nmi() {
        nmi(cpu, bus);
    }

    cycles
}
```

### NES Timing

The NES runs on a ~21.47727 MHz master clock, divided into:

| Component | Divisor | Frequency | Ratio per CPU cycle |
|-----------|---------|-----------|-------------------|
| CPU | /12 | ~1.79 MHz | 1 |
| PPU | /4 | ~5.37 MHz | 3 |
| APU | /12 | ~1.79 MHz | 1 |

So for every CPU instruction, the PPU advances by `cpu_cycles * 3` dots.

## Module Responsibilities

| Module | Role |
|--------|------|
| `cpu.rs` | Register file, flag manipulation |
| `ops.rs` | Instruction decoding and execution |
| `bus.rs` | Address routing, hardware interaction |
| `ppu.rs` | Video generation, NMI control |
| `apu.rs` | Audio generation |
| `rom.rs` | ROM parsing and bank switching |

## no_std Design

The entire emulator runs without the Rust standard library. This means:

- No `Vec`, `Box`, `String` — ROM data is borrowed as `&[u8]`
- No heap allocation — all state is fixed-size stack-allocated
- No OS dependencies — works on bare metal
- Custom panic handler — infinite loop instead of stack unwinding

```rust
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
```

This makes the emulator usable in kernels, firmware, and microcontrollers where `std` isn't available.
