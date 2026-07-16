use crate::bus::Bus;
use crate::cpu::CpuRp2a03;
use crate::ops::addr_modes;

// ---- SHA (Store A & X & H at address) ----
// H is derived from the data bus after reading the operand's high byte.
// The 6502 bus always holds (high_byte + 1) after a 16-bit operand read.
// Stored value = A & X & H (where H = hi + 1, or 0xFF if DMC DMA just fired).
// On page cross: stored address high byte = H & X.
pub fn sha_indy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let ptr = bus.read(cpu.pc()) as u16;
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    // Zero-page wrap: when ptr = $FF, high byte is read from $00, not $100
    let hi_ptr = (ptr as u8).wrapping_add(1) as u16;
    let base = bus.read(ptr) as u16;
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    let hi = bus.read(hi_ptr) as u16;
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    let base = base | (hi << 8);
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let addr = base.wrapping_add(cpu.y() as u16);
    let page = (((base ^ addr) >> 8) as u8) & 1;
    // H = high byte of pointer target + 1 (from data bus after 16-bit read)
    let h = ((base >> 8) as u8).wrapping_add(1);
    // Check DMC: dmc_just_fired from previous dmc_tick, OR dma became pending
    // during this instruction's bus accesses (ticked above)
    let ignore_h = core::mem::replace(&mut bus.dmc_just_fired, false) || bus.apu.dmc_dma_pending();
    let final_h = if ignore_h { 0xFF } else { h };
    let val = cpu.a() & cpu.x() & final_h;
    let _ = bus.read(addr);
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    let final_addr = if page != 0 {
        (addr as u8 as u16) | ((h & cpu.x()) as u16) << 8
    } else {
        addr
    };
    bus.write(final_addr, val);
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    6
}

pub fn sha_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let lo = bus.read(cpu.pc()) as u16;
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    let hi = bus.read(cpu.pc().wrapping_add(1)) as u16;
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    let base = lo | (hi << 8);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let addr = base.wrapping_add(cpu.y() as u16);
    let page = (((base ^ addr) >> 8) as u8) & 1;
    // H = operand high byte + 1 (from data bus after reading hi)
    let h = (hi as u8).wrapping_add(1);
    let ignore_h = core::mem::replace(&mut bus.dmc_just_fired, false) || bus.apu.dmc_dma_pending();
    let final_h = if ignore_h { 0xFF } else { h };
    let val = cpu.a() & cpu.x() & final_h;
    let _ = bus.read(addr);
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    let final_addr = if page != 0 {
        (addr as u8 as u16) | ((h & cpu.x()) as u16) << 8
    } else {
        addr
    };
    bus.write(final_addr, val);
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    5
}

// ---- SHS (Same as SHA abs,y but also sets SP = A & X) ----
pub fn shs_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let lo = bus.read(cpu.pc()) as u16;
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    let hi = bus.read(cpu.pc().wrapping_add(1)) as u16;
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    let base = lo | (hi << 8);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let addr = base.wrapping_add(cpu.y() as u16);
    let page = (((base ^ addr) >> 8) as u8) & 1;
    let sp_val = cpu.a() & cpu.x();
    cpu.set_st(sp_val);
    // H = operand high byte + 1 (from data bus after reading hi)
    let h = (hi as u8).wrapping_add(1);
    let ignore_h = core::mem::replace(&mut bus.dmc_just_fired, false) || bus.apu.dmc_dma_pending();
    let final_h = if ignore_h { 0xFF } else { h };
    let val = sp_val & final_h;
    let _ = bus.read(addr);
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    let final_addr = if page != 0 {
        (addr as u8 as u16) | ((h & cpu.x()) as u16) << 8
    } else {
        addr
    };
    bus.write(final_addr, val);
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    5
}

// ---- SHY (Store Y & H at address) ----
// H = operand high byte + 1 (from data bus after reading hi).
pub fn shy_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let lo = bus.read(cpu.pc()) as u16;
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    let hi = bus.read(cpu.pc().wrapping_add(1)) as u16;
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    let base = lo | (hi << 8);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let addr = base.wrapping_add(cpu.x() as u16);
    let page = (((base ^ addr) >> 8) as u8) & 1;
    // H = operand high byte + 1 (from data bus after reading hi)
    let h = (hi as u8).wrapping_add(1);
    let ignore_h = core::mem::replace(&mut bus.dmc_just_fired, false) || bus.apu.dmc_dma_pending();
    let final_h = if ignore_h { 0xFF } else { h };
    let val = cpu.y() & final_h;
    let _ = bus.read(addr);
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    let final_addr = if page != 0 {
        (addr as u8 as u16) | ((h & cpu.y()) as u16) << 8
    } else {
        addr
    };
    bus.write(final_addr, val);
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    5
}

// ---- SHX (Store X & H at address) ----
// H = operand high byte + 1 (from data bus after reading hi).
pub fn shx_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let lo = bus.read(cpu.pc()) as u16;
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    let hi = bus.read(cpu.pc().wrapping_add(1)) as u16;
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    let base = lo | (hi << 8);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let addr = base.wrapping_add(cpu.y() as u16);
    let page = (((base ^ addr) >> 8) as u8) & 1;
    // H = operand high byte + 1 (from data bus after reading hi)
    let h = (hi as u8).wrapping_add(1);
    let ignore_h = core::mem::replace(&mut bus.dmc_just_fired, false) || bus.apu.dmc_dma_pending();
    let final_h = if ignore_h { 0xFF } else { h };
    let val = cpu.x() & final_h;
    let _ = bus.read(addr);
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    let final_addr = if page != 0 {
        (addr as u8 as u16) | ((h & cpu.x()) as u16) << 8
    } else {
        addr
    };
    bus.write(final_addr, val);
    bus.apu.tick_dmc();
    bus.dmc_ticks += 1;
    5
}

// ---- LAE (A = SP & operand, X = A, SP = A) ----
pub fn lae_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    let (addr, page) = addr_modes::absy(cpu, bus);
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let val = cpu.st() & bus.read(addr);
    cpu.set_a(val);
    cpu.set_x(val);
    cpu.set_st(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    4 + page
}
