use crate::bus::Bus;
use crate::cpu::{CpuRp2a03, FLAG_CARRY, FLAG_NEGATIVE, FLAG_OVERFLOW, FLAG_ZERO};

fn branch(cpu: &mut CpuRp2a03, bus: &mut Bus, cond: bool) -> u8 {
    let disp = bus.read(cpu.pc()) as i8 as u16;
    cpu.set_pc(cpu.pc().wrapping_add(1));
    if cond {
        let old_pc = cpu.pc();
        let new_pc = old_pc.wrapping_add(disp);
        cpu.set_pc(new_pc);
        2 + 1 + (((old_pc ^ new_pc) >> 8) as u8 & 1)
    } else {
        2
    }
}

pub fn bpl(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    branch(cpu, bus, !cpu.get_flag(FLAG_NEGATIVE))
}
pub fn bmi(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    branch(cpu, bus, cpu.get_flag(FLAG_NEGATIVE))
}
pub fn bvc(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    branch(cpu, bus, !cpu.get_flag(FLAG_OVERFLOW))
}
pub fn bvs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    branch(cpu, bus, cpu.get_flag(FLAG_OVERFLOW))
}
pub fn bcc(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    branch(cpu, bus, !cpu.get_flag(FLAG_CARRY))
}
pub fn bcs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    branch(cpu, bus, cpu.get_flag(FLAG_CARRY))
}
pub fn bne(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    branch(cpu, bus, !cpu.get_flag(FLAG_ZERO))
}
pub fn beq(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    branch(cpu, bus, cpu.get_flag(FLAG_ZERO))
}
