use crate::bus::Bus;
use crate::cpu::CpuRp2a03;
use crate::ops::addr_modes;

pub fn inc_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    let result = val.wrapping_add(1);
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    5
}

pub fn inc_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    let result = val.wrapping_add(1);
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    6
}

pub fn inc_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    let result = val.wrapping_add(1);
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    6
}

pub fn inc_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, _) = addr_modes::absx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    let result = val.wrapping_add(1);
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    7
}

pub fn dec_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    let result = val.wrapping_sub(1);
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    5
}

pub fn dec_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    let result = val.wrapping_sub(1);
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    6
}

pub fn dec_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    let result = val.wrapping_sub(1);
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    6
}

pub fn dec_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, _) = addr_modes::absx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = bus.read(addr);
    bus.write(addr, val); // Dummy write (write back original value)
    let result = val.wrapping_sub(1);
    bus.write(addr, result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    7
}

pub fn dex(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
    let result = cpu.x().wrapping_sub(1);
    cpu.set_x(result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    2
}

pub fn dey(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
    let result = cpu.y().wrapping_sub(1);
    cpu.set_y(result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    2
}

pub fn inx(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
    let result = cpu.x().wrapping_add(1);
    cpu.set_x(result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    2
}

pub fn iny(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
    let result = cpu.y().wrapping_add(1);
    cpu.set_y(result);
    cpu.set_sign(result);
    cpu.set_zero(result);
    2
}
