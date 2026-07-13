#![cfg_attr(not(test), no_std)]
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
    clippy::missing_const_for_fn
)]

pub mod address;
pub mod apu;
pub mod bus;
pub mod clock;
pub mod cpu;
pub mod interrupt;
pub mod ops;
pub mod ppu;
pub mod rom;

use bus::Bus;
use cpu::{Cpu6502, FLAG_INTERRUPT};
use ops::TABLE;

pub fn tick(cpu: &mut Cpu6502, bus: &mut Bus<'_>) -> u8 {
    let opcode = bus.read(cpu.pc());
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let cycles = TABLE[opcode as usize](cpu, bus);

    bus.ppu.tick_batch((cycles as u16) * 3);
    bus.apu.tick(cycles);

    if bus.poll_nmi() {
        nmi(cpu, bus);
    }

    cycles
}

pub fn nmi(cpu: &mut Cpu6502, bus: &mut Bus<'_>) {
    crate::ops::push(cpu, bus, (cpu.pc() >> 8) as u8);
    crate::ops::push(cpu, bus, cpu.pc() as u8);
    let sr = cpu.sr() | 0x20;
    crate::ops::push(cpu, bus, sr);
    cpu.set_flag(FLAG_INTERRUPT, true);
    let lo = bus.read(0xFFFA) as u16;
    let hi = bus.read(0xFFFB) as u16;
    cpu.set_pc(lo | (hi << 8));
}

pub fn reset(cpu: &mut Cpu6502, bus: &mut Bus<'_>) {
    let lo = bus.read(0xFFFC) as u16;
    let hi = bus.read(0xFFFD) as u16;
    *cpu = Cpu6502::new(lo | (hi << 8));
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
