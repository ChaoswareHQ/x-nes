use crate::bus::Bus;
use crate::cpu::{CpuRp2a03, FLAG_CARRY, FLAG_OVERFLOW, FLAG_ZERO};
use crate::ops::addr_modes;

pub fn and_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let val = bus.read(cpu.pc()) & cpu.a();
    cpu.set_pc(cpu.pc().wrapping_add(1));
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    2
}

pub fn and_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr) & cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    3
}

pub fn and_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr) & cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    4
}

pub fn and_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr) & cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    4
}

pub fn and_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::absx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr) & cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    4 + page
}

pub fn and_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::absy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr) & cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    4 + page
}

pub fn and_indx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::indx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr) & cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    6
}

pub fn and_indy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::indy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr) & cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    5 + page
}

pub fn ora_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let val = bus.read(cpu.pc()) | cpu.a();
    cpu.set_pc(cpu.pc().wrapping_add(1));
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    2
}

pub fn ora_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr) | cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    3
}

pub fn ora_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr) | cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    4
}

pub fn ora_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr) | cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    4
}

pub fn ora_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::absx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr) | cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    4 + page
}

pub fn ora_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::absy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr) | cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    4 + page
}

pub fn ora_indx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::indx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr) | cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    6
}

pub fn ora_indy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::indy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr) | cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    5 + page
}

pub fn eor_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let val = bus.read(cpu.pc()) ^ cpu.a();
    cpu.set_pc(cpu.pc().wrapping_add(1));
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    2
}

pub fn eor_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr) ^ cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    3
}

pub fn eor_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr) ^ cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    4
}

pub fn eor_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr) ^ cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    4
}

pub fn eor_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::absx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr) ^ cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    4 + page
}

pub fn eor_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::absy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr) ^ cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    4 + page
}

pub fn eor_indx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::indx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr) ^ cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    6
}

pub fn eor_indy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::indy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr) ^ cpu.a();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    5 + page
}

pub fn bit_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr);
    cpu.set_sign(val);
    cpu.set_flag(FLAG_OVERFLOW, val & 0x40 != 0);
    cpu.set_flag(FLAG_ZERO, cpu.a() & val == 0);
    3
}

pub fn bit_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr);
    cpu.set_sign(val);
    cpu.set_flag(FLAG_OVERFLOW, val & 0x40 != 0);
    cpu.set_flag(FLAG_ZERO, cpu.a() & val == 0);
    4
}

// ---- ANC (AND with A, then copy N flag to C) ----
pub fn anc_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let val = bus.read(cpu.pc()) & cpu.a();
    cpu.set_pc(cpu.pc().wrapping_add(1));
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
    2
}

// ---- ANE (A = (A | constant) & X & operand) ----
pub fn ane_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let operand = bus.read(cpu.pc());
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = (cpu.a() | 0xEE) & cpu.x() & operand;
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    2
}

// ---- LXA (A = (A | constant) & operand, X = A) ----
pub fn lxa_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let operand = bus.read(cpu.pc());
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = (cpu.a() | 0xEE) & operand;
    cpu.set_a(val);
    cpu.set_x(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    2
}
