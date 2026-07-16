#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    unsafe_op_in_unsafe_fn,
    dead_code,
    unused_imports
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::module_name_repetitions,
    clippy::similar_names,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::items_after_statements,
    clippy::wildcard_enum_match_arm,
    clippy::inline_always,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::wildcard_imports,
    clippy::elidable_lifetime_names,
    clippy::large_stack_arrays,
    clippy::derivable_impls,
    clippy::collapsible_match,
    clippy::manual_range_contains,
    clippy::match_same_arms,
    clippy::new_without_default,
    clippy::missing_const_for_fn,
    clippy::struct_excessive_bools
)]

pub mod address;
pub mod apu;
pub mod bus;
pub mod clock;
pub mod cpu;

#[cfg(feature = "ffi")]
pub mod ffi;

pub mod controller;
pub mod interrupt;
pub mod mapper;
pub mod ops;
pub mod ppu;
pub mod rom;

use bus::Bus;
use cpu::{CpuRp2a03, FLAG_BREAK, FLAG_INTERRUPT};
use ops::{BASE_CYCLES, TABLE};

pub fn tick(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    // Promote deferred NMI at instruction boundary.
    // On real NES, enabling NMI ($2000 write) during VBlank fires NMI
    // after the NEXT instruction, not immediately.
    // The write_ctrl sets nmi_deferred during instruction N;
    // we promote it here at the start of instruction N+1 so that
    // NMI fires at the end of instruction N+1.
    if bus.ppu.nmi_deferred {
        bus.ppu.nmi_pending = true;
        bus.ppu.nmi_deferred = false;
    }

    let opcode = bus.read(cpu.pc());
    cpu.set_pc(cpu.pc().wrapping_add(1));

    // Pre-tick APU by base cycles BEFORE instruction execution,
    // so DMC timer expiry is detected and dmc_just_fired is set
    // before the instruction's write cycle.
    let base = BASE_CYCLES[opcode as usize];
    bus.apu.tick(base as u16);
    if bus.apu.dmc_dma_pending() {
        bus.dmc_just_fired = true;
    }

    // Now execute the instruction (it sees dmc_just_fired if DMA is pending)
    let mut cycles = TABLE[opcode as usize](cpu, bus);

    // Tick PPU for actual instruction cycles
    bus.ppu_tick((cycles as u16) * 3);

    // Tick remaining APU cycles (those beyond base cycles, e.g. page cross)
    if (cycles as u16) > base as u16 {
        bus.apu.tick((cycles as u16) - base as u16);
    }

    // Handle DMC DMA if pending (runs between instructions)
    cycles += bus.dmc_tick();

    // Check for interrupts after instruction + DMA complete.
    // NMI is edge-triggered: sampled at instruction boundaries.
    if bus.poll_nmi() {
        nmi(cpu, bus);
    } else if !cpu.get_flag(FLAG_INTERRUPT) && bus.poll_irq() {
        irq(cpu, bus);
    }

    cycles
}

pub fn nmi(cpu: &mut CpuRp2a03, bus: &mut Bus) {
    // NMI takes 7 CPU cycles on the NES (21 PPU cycles)
    // Push PCH (2), Push PCL (2), Push SR (2), Read vector (1)
    crate::ops::push(cpu, bus, (cpu.pc() >> 8) as u8);
    crate::ops::push(cpu, bus, cpu.pc() as u8);
    // NMI pushes with B flag CLEAR (bit 4 = 0), bit 5 always SET
    let sr = (cpu.sr() & !FLAG_BREAK) | 0x20;
    crate::ops::push(cpu, bus, sr);
    cpu.set_flag(FLAG_INTERRUPT, true);
    let lo = bus.read(0xFFFA) as u16;
    let hi = bus.read(0xFFFB) as u16;
    cpu.set_pc(lo | (hi << 8));
    // Advance PPU by 21 cycles to account for NMI handling
    bus.ppu_tick(21);
}

pub fn irq(cpu: &mut CpuRp2a03, bus: &mut Bus) {
    // IRQ takes 7 CPU cycles on the NES
    // Push PCH, PCL, SR (B flag CLEAR, bit 5 SET), Read vector
    crate::ops::push(cpu, bus, (cpu.pc() >> 8) as u8);
    crate::ops::push(cpu, bus, cpu.pc() as u8);
    let sr = (cpu.sr() & !FLAG_BREAK) | 0x20;
    crate::ops::push(cpu, bus, sr);
    cpu.set_flag(FLAG_INTERRUPT, true);
    let lo = bus.read(0xFFFE) as u16;
    let hi = bus.read(0xFFFF) as u16;
    cpu.set_pc(lo | (hi << 8));
    bus.ppu_tick(21);
}

pub fn reset(cpu: &mut CpuRp2a03, bus: &mut Bus) {
    let lo = bus.read(0xFFFC) as u16;
    let hi = bus.read(0xFFFD) as u16;
    *cpu = CpuRp2a03::new(lo | (hi << 8));
}

#[cfg(not(any(test, feature = "std")))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
