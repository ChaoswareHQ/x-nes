use crate::bus::Bus;
use crate::cpu::{CpuRp2a03, FLAG_CARRY, FLAG_OVERFLOW};
use crate::ops::addr_modes;

pub fn adc_inner(cpu: &mut CpuRp2a03, operand: u8) {
    let a = cpu.a();
    let carry = cpu.get_flag(FLAG_CARRY) as u16;
    let result = a as u16 + operand as u16 + carry;
    let lo = result as u8;
    // NES RP2A03 does NOT support decimal mode for ADC/SBC
    cpu.set_sign(lo);
    cpu.set_zero(lo);
    cpu.set_flag(FLAG_CARRY, result > 0xFF);
    // ADC overflow: set when A and operand have same sign but result has opposite sign
    cpu.set_flag(
        FLAG_OVERFLOW,
        ((a ^ operand) & 0x80) == 0 && ((a ^ lo) & 0x80) != 0,
    );
    cpu.set_a(lo);
}

pub fn adc_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let operand = bus.read(cpu.pc());
    cpu.set_pc(cpu.pc().wrapping_add(1));
    adc_inner(cpu, operand);
    2
}

pub fn adc_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    adc_inner(cpu, bus.read(addr));
    3
}

pub fn adc_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    adc_inner(cpu, bus.read(addr));
    4
}

pub fn adc_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    adc_inner(cpu, bus.read(addr));
    4
}

pub fn adc_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::absx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    adc_inner(cpu, bus.read(addr));
    4 + page
}

pub fn adc_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::absy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    adc_inner(cpu, bus.read(addr));
    4 + page
}

pub fn adc_indx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::indx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    adc_inner(cpu, bus.read(addr));
    6
}

pub fn adc_indy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::indy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    adc_inner(cpu, bus.read(addr));
    5 + page
}

pub fn sbc_inner(cpu: &mut CpuRp2a03, operand: u8) {
    let a = cpu.a();
    let carry = cpu.get_flag(FLAG_CARRY) as u16;
    let result = (a as u16)
        .wrapping_sub(operand as u16)
        .wrapping_sub(1 - carry);
    let lo = result as u8;
    // NES RP2A03 does NOT support decimal mode for ADC/SBC
    cpu.set_sign(lo);
    cpu.set_zero(lo);
    cpu.set_flag(FLAG_CARRY, result < 0x100);
    cpu.set_flag(
        FLAG_OVERFLOW,
        ((a ^ lo) & 0x80) != 0 && ((a ^ operand) & 0x80) != 0,
    );
    cpu.set_a(lo);
}

pub fn sbc_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let operand = bus.read(cpu.pc());
    cpu.set_pc(cpu.pc().wrapping_add(1));
    sbc_inner(cpu, operand);
    2
}

pub fn sbc_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    sbc_inner(cpu, bus.read(addr));
    3
}

pub fn sbc_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    sbc_inner(cpu, bus.read(addr));
    4
}

pub fn sbc_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    sbc_inner(cpu, bus.read(addr));
    4
}

pub fn sbc_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::absx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    sbc_inner(cpu, bus.read(addr));
    4 + page
}

pub fn sbc_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::absy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    sbc_inner(cpu, bus.read(addr));
    4 + page
}

pub fn sbc_indx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::indx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    sbc_inner(cpu, bus.read(addr));
    6
}

pub fn sbc_indy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::indy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    sbc_inner(cpu, bus.read(addr));
    5 + page
}

pub fn cmp_inner(cpu: &mut CpuRp2a03, reg: u8, mem: u8) {
    let diff = reg.wrapping_sub(mem);
    cpu.set_flag(FLAG_CARRY, reg >= mem);
    cpu.set_sign(diff);
    cpu.set_zero(diff);
}

pub fn cmp_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let val = bus.read(cpu.pc());
    cpu.set_pc(cpu.pc().wrapping_add(1));
    cmp_inner(cpu, cpu.a(), val);
    2
}

pub fn cmp_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    cmp_inner(cpu, cpu.a(), bus.read(addr));
    3
}

pub fn cmp_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    cmp_inner(cpu, cpu.a(), bus.read(addr));
    4
}

pub fn cmp_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    cmp_inner(cpu, cpu.a(), bus.read(addr));
    4
}

pub fn cmp_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::absx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    cmp_inner(cpu, cpu.a(), bus.read(addr));
    4 + page
}

pub fn cmp_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::absy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    cmp_inner(cpu, cpu.a(), bus.read(addr));
    4 + page
}

pub fn cmp_indx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::indx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    cmp_inner(cpu, cpu.a(), bus.read(addr));
    6
}

pub fn cmp_indy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::indy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    cmp_inner(cpu, cpu.a(), bus.read(addr));
    5 + page
}

pub fn cpx_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let val = bus.read(cpu.pc());
    cpu.set_pc(cpu.pc().wrapping_add(1));
    cmp_inner(cpu, cpu.x(), val);
    2
}

pub fn cpx_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    cmp_inner(cpu, cpu.x(), bus.read(addr));
    3
}

pub fn cpx_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    cmp_inner(cpu, cpu.x(), bus.read(addr));
    4
}

pub fn cpy_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let val = bus.read(cpu.pc());
    cpu.set_pc(cpu.pc().wrapping_add(1));
    cmp_inner(cpu, cpu.y(), val);
    2
}

pub fn cpy_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    cmp_inner(cpu, cpu.y(), bus.read(addr));
    3
}

pub fn cpy_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    cmp_inner(cpu, cpu.y(), bus.read(addr));
    4
}
