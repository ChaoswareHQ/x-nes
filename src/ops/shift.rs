use crate::bus::Bus;
use crate::cpu::{CpuRp2a03, FLAG_CARRY};
use crate::ops::addr_modes;

pub fn asl_a(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
    let val = cpu.a();
    cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
    let result = val << 1;
    cpu.set_a(result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    2
}

pub fn asl_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
    let result = val << 1;
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    5
}

pub fn asl_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
    let result = val << 1;
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    6
}

pub fn asl_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
    let result = val << 1;
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    6
}

pub fn asl_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, _) = addr_modes::absx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
    let result = val << 1;
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    7
}

pub fn lsr_a(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
    let val = cpu.a();
    cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
    let result = val >> 1;
    cpu.set_a(result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    2
}

pub fn lsr_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
    let result = val >> 1;
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    5
}

pub fn lsr_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
    let result = val >> 1;
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    6
}

pub fn lsr_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
    let result = val >> 1;
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    6
}

pub fn lsr_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, _) = addr_modes::absx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
    let result = val >> 1;
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    7
}

pub fn rol_a(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
    let val = cpu.a();
    let carry = cpu.get_flag(FLAG_CARRY) as u8;
    cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
    let result = (val << 1) | carry;
    cpu.set_a(result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    2
}

pub fn rol_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    let carry = cpu.get_flag(FLAG_CARRY) as u8;
    cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
    let result = (val << 1) | carry;
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    5
}

pub fn rol_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    let carry = cpu.get_flag(FLAG_CARRY) as u8;
    cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
    let result = (val << 1) | carry;
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    6
}

pub fn rol_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    let carry = cpu.get_flag(FLAG_CARRY) as u8;
    cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
    let result = (val << 1) | carry;
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    6
}

pub fn rol_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, _) = addr_modes::absx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    let carry = cpu.get_flag(FLAG_CARRY) as u8;
    cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
    let result = (val << 1) | carry;
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    7
}

pub fn ror_a(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
    let val = cpu.a();
    let carry = cpu.get_flag(FLAG_CARRY) as u8;
    cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
    let result = (val >> 1) | (carry << 7);
    cpu.set_a(result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    2
}

pub fn ror_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    let carry = cpu.get_flag(FLAG_CARRY) as u8;
    cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
    let result = (val >> 1) | (carry << 7);
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    5
}

pub fn ror_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    let carry = cpu.get_flag(FLAG_CARRY) as u8;
    cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
    let result = (val >> 1) | (carry << 7);
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    6
}

pub fn ror_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    let carry = cpu.get_flag(FLAG_CARRY) as u8;
    cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
    let result = (val >> 1) | (carry << 7);
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    6
}

pub fn ror_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, _) = addr_modes::absx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    let carry = cpu.get_flag(FLAG_CARRY) as u8;
    cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
    let result = (val >> 1) | (carry << 7);
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    7
}
