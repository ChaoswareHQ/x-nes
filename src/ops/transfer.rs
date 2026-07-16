use crate::bus::Bus;
use crate::cpu::CpuRp2a03;
use crate::ops::addr_modes;

fn lda_inner(cpu: &mut CpuRp2a03, val: u8) {
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
}

pub fn lda_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let val = bus.read(cpu.pc());
    cpu.set_pc(cpu.pc().wrapping_add(1));
    lda_inner(cpu, val);
    2
}

pub fn lda_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    lda_inner(cpu, bus.read(addr));
    3
}

pub fn lda_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    lda_inner(cpu, bus.read(addr));
    4
}

pub fn lda_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    lda_inner(cpu, bus.read(addr));
    4
}

pub fn lda_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::absx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    lda_inner(cpu, bus.read(addr));
    4 + page
}

pub fn lda_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::absy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    lda_inner(cpu, bus.read(addr));
    4 + page
}

pub fn lda_indx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::indx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    lda_inner(cpu, bus.read(addr));
    6
}

pub fn lda_indy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::indy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    lda_inner(cpu, bus.read(addr));
    5 + page
}

pub fn sta_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    bus.write(addr, cpu.a());
    3
}

pub fn sta_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    bus.write(addr, cpu.a());
    4
}

pub fn sta_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    bus.write(addr, cpu.a());
    4
}

pub fn sta_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, _page) = addr_modes::absx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let _ = bus.read(addr); // Dummy read before write
    bus.write(addr, cpu.a());
    5
}

pub fn sta_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, _) = addr_modes::absy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let _ = bus.read(addr); // Dummy read before write
    bus.write(addr, cpu.a());
    5
}

pub fn sta_indx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::indx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    bus.write(addr, cpu.a());
    6
}

pub fn sta_indy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, _) = addr_modes::indy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let _ = bus.read(addr); // Dummy read before write
    bus.write(addr, cpu.a());
    6
}

fn ldx_inner(cpu: &mut CpuRp2a03, val: u8) {
    cpu.set_x(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
}

pub fn ldx_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let val = bus.read(cpu.pc());
    cpu.set_pc(cpu.pc().wrapping_add(1));
    ldx_inner(cpu, val);
    2
}

pub fn ldx_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    ldx_inner(cpu, bus.read(addr));
    3
}

pub fn ldx_zpy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    ldx_inner(cpu, bus.read(addr));
    4
}

pub fn ldx_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    ldx_inner(cpu, bus.read(addr));
    4
}

pub fn ldx_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::absy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    ldx_inner(cpu, bus.read(addr));
    4 + page
}

fn ldy_inner(cpu: &mut CpuRp2a03, val: u8) {
    cpu.set_y(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
}

pub fn ldy_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let val = bus.read(cpu.pc());
    cpu.set_pc(cpu.pc().wrapping_add(1));
    ldy_inner(cpu, val);
    2
}

pub fn ldy_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    ldy_inner(cpu, bus.read(addr));
    3
}

pub fn ldy_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    ldy_inner(cpu, bus.read(addr));
    4
}

pub fn ldy_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    ldy_inner(cpu, bus.read(addr));
    4
}

pub fn ldy_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::absx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    ldy_inner(cpu, bus.read(addr));
    4 + page
}

pub fn stx_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    bus.write(addr, cpu.x());
    3
}

pub fn stx_zpy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    bus.write(addr, cpu.x());
    4
}

pub fn stx_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    bus.write(addr, cpu.x());
    4
}

pub fn sty_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zp(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    bus.write(addr, cpu.y());
    3
}

pub fn sty_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::zpx(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    bus.write(addr, cpu.y());
    4
}

pub fn sty_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let addr = addr_modes::abs(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    bus.write(addr, cpu.y());
    4
}

pub fn tax(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
    let val = cpu.a();
    cpu.set_x(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    2
}

pub fn txa(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
    let val = cpu.x();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    2
}

pub fn tay(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
    let val = cpu.a();
    cpu.set_y(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    2
}

pub fn tya(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
    let val = cpu.y();
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    2
}

pub fn tsx(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
    let val = cpu.st();
    cpu.set_x(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    2
}

pub fn txs(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
    cpu.set_st(cpu.x());
    2
}
