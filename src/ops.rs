use crate::bus::Bus;
use crate::cpu::{
    CpuRp2a03, FLAG_CARRY, FLAG_DECIMAL, FLAG_INTERRUPT, FLAG_NEGATIVE, FLAG_OVERFLOW, FLAG_ZERO,
};

type Op = fn(&mut CpuRp2a03, &mut Bus) -> u8;

#[inline(always)]
pub(crate) fn push(cpu: &mut CpuRp2a03, bus: &mut Bus, val: u8) {
    bus.write(0x0100 | cpu.st() as u16, val);
    cpu.set_st(cpu.st().wrapping_sub(1));
}

#[inline(always)]
pub(crate) fn pull(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
    cpu.set_st(cpu.st().wrapping_add(1));
    bus.read(0x0100 | cpu.st() as u16)
}

mod addr_modes {
    use crate::bus::Bus;
    use crate::cpu::CpuRp2a03;

    #[inline(always)]
    pub fn abs(cpu: &CpuRp2a03, bus: &mut Bus) -> u16 {
        let lo = bus.read(cpu.pc()) as u16;
        let hi = bus.read(cpu.pc().wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    #[inline(always)]
    pub fn absx(cpu: &CpuRp2a03, bus: &mut Bus) -> (u16, u8) {
        let base = abs(cpu, bus);
        let addr = base.wrapping_add(cpu.x() as u16);
        let page = (((base ^ addr) >> 8) as u8) & 1;
        // Dummy read when page boundary is crossed
        if page != 0 {
            let dummy_addr = (base & 0xFF00) | (addr as u8 as u16);
            bus.read(dummy_addr);
        }
        (addr, page)
    }

    #[inline(always)]
    pub fn absy(cpu: &CpuRp2a03, bus: &mut Bus) -> (u16, u8) {
        let base = abs(cpu, bus);
        let addr = base.wrapping_add(cpu.y() as u16);
        let page = (((base ^ addr) >> 8) as u8) & 1;
        // Dummy read when page boundary is crossed
        if page != 0 {
            let dummy_addr = (base & 0xFF00) | (addr as u8 as u16);
            bus.read(dummy_addr);
        }
        (addr, page)
    }

    #[inline(always)]
    pub fn zp(cpu: &CpuRp2a03, bus: &mut Bus) -> u16 {
        bus.read(cpu.pc()) as u16
    }

    #[inline(always)]
    pub fn zpx(cpu: &CpuRp2a03, bus: &mut Bus) -> u16 {
        bus.read(cpu.pc()).wrapping_add(cpu.x()) as u16
    }

    #[inline(always)]
    pub fn zpy(cpu: &CpuRp2a03, bus: &mut Bus) -> u16 {
        bus.read(cpu.pc()).wrapping_add(cpu.y()) as u16
    }

    #[inline(always)]
    pub fn indx(cpu: &CpuRp2a03, bus: &mut Bus) -> u16 {
        let ptr = bus.read(cpu.pc()).wrapping_add(cpu.x()) as u16;
        let lo = bus.read(ptr) as u16;
        // Zero-page wrap: when ptr = $FF, read high byte from $00, not $100
        let hi_ptr = (ptr as u8).wrapping_add(1) as u16;
        let hi = bus.read(hi_ptr) as u16;
        lo | (hi << 8)
    }

    #[inline(always)]
    pub fn indy(cpu: &CpuRp2a03, bus: &mut Bus) -> (u16, u8) {
        let ptr = bus.read(cpu.pc()) as u16;
        // Zero-page wrap: when ptr = $FF, read high byte from $00, not $100
        let hi_ptr = (ptr as u8).wrapping_add(1) as u16;
        let base = bus.read(ptr) as u16 | (bus.read(hi_ptr) as u16) << 8;
        let addr = base.wrapping_add(cpu.y() as u16);
        let page = (((base ^ addr) >> 8) as u8) & 1;
        // Dummy read when page boundary is crossed
        if page != 0 {
            let dummy_addr = (base & 0xFF00) | (addr as u8 as u16);
            bus.read(dummy_addr);
        }
        (addr, page)
    }
}

pub static TABLE: [Op; 256] = {
    use self::op::*;
    [
        // 0x00-0x0F
        brk, ora_indx, nop_imm, slo_indx, nop_zp, ora_zp, asl_zp, slo_zp, php, ora_imm, asl_a,
        anc_imm, nop_abs, ora_abs, asl_abs, slo_abs, // 0x10-0x1F
        bpl, ora_indy, nop_imm, slo_indy, nop_zpx, ora_zpx, asl_zpx, slo_zpx, clc, ora_absy, nop,
        slo_absy, nop_absx, ora_absx, asl_absx, slo_absx, // 0x20-0x2F
        jsr, and_indx, nop_imm, rla_indx, bit_zp, and_zp, rol_zp, rla_zp, plp, and_imm, rol_a,
        anc_imm, bit_abs, and_abs, rol_abs, rla_abs, // 0x30-0x3F
        bmi, and_indy, nop_imm, rla_indy, nop_zpx, and_zpx, rol_zpx, rla_zpx, sec, and_absy, nop,
        rla_absy, nop_absx, and_absx, rol_absx, rla_absx, // 0x40-0x4F
        rti, eor_indx, nop_imm, sre_indx, nop_zp, eor_zp, lsr_zp, sre_zp, pha, eor_imm, lsr_a,
        asr_imm, jmp_abs, eor_abs, lsr_abs, sre_abs, // 0x50-0x5F
        bvc, eor_indy, nop_imm, sre_indy, nop_zpx, eor_zpx, lsr_zpx, sre_zpx, cli, eor_absy, nop,
        sre_absy, nop_absx, eor_absx, lsr_absx, sre_absx, // 0x60-0x6F
        rts, adc_indx, nop_imm, rra_indx, nop_zp, adc_zp, ror_zp, rra_zp, pla, adc_imm, ror_a,
        arr_imm, jmp_ind, adc_abs, ror_abs, rra_abs, // 0x70-0x7F
        bvs, adc_indy, nop_imm, rra_indy, nop_zpx, adc_zpx, ror_zpx, rra_zpx, sei, adc_absy, nop,
        rra_absy, nop_absx, adc_absx, ror_absx, rra_absx, // 0x80-0x8F
        nop_imm, sta_indx, nop_imm, sax_indx, sty_zp, sta_zp, stx_zp, sax_zp, dey, nop_imm, txa,
        ane_imm, sty_abs, sta_abs, stx_abs, sax_abs, // 0x90-0x9F
        bcc, sta_indy, nop_imm, sha_indy, sty_zpx, sta_zpx, stx_zpy, sax_zpy, tya, sta_absy, txs,
        shs_absy, shy_absx, sta_absx, shx_absy, sha_absy, // 0xA0-0xAF
        ldy_imm, lda_indx, ldx_imm, lax_indx, ldy_zp, lda_zp, ldx_zp, lax_zp, tay, lda_imm, tax,
        lxa_imm, ldy_abs, lda_abs, ldx_abs, lax_abs, // 0xB0-0xBF
        bcs, lda_indy, nop_imm, lax_indy, ldy_zpx, lda_zpx, ldx_zpy, lax_zpy, clv, lda_absy, tsx,
        lae_absy, ldy_absx, lda_absx, ldx_absy, lax_absy, // 0xC0-0xCF
        cpy_imm, cmp_indx, nop_imm, dcp_indx, cpy_zp, cmp_zp, dec_zp, dcp_zp, iny, cmp_imm, dex,
        axs_imm, cpy_abs, cmp_abs, dec_abs, dcp_abs, // 0xD0-0xDF
        bne, cmp_indy, nop_imm, dcp_indy, nop_zpx, cmp_zpx, dec_zpx, dcp_zpx, cld, cmp_absy, nop,
        dcp_absy, nop_absx, cmp_absx, dec_absx, dcp_absx, // 0xE0-0xEF
        cpx_imm, sbc_indx, nop_imm, isc_indx, cpx_zp, sbc_zp, inc_zp, isc_zp, inx, sbc_imm, nop,
        sbc_imm, cpx_abs, sbc_abs, inc_abs, isc_abs, // 0xF0-0xFF
        beq, sbc_indy, nop_imm, isc_indy, nop_zpx, sbc_zpx, inc_zpx, isc_zpx, sed, sbc_absy, nop,
        isc_absy, nop_absx, sbc_absx, inc_absx, isc_absx,
    ]
};

mod op {
    use super::*;
    use crate::cpu::FLAG_BREAK;

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

    // ---- STA (Indirect), Y ----
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

    fn adc_inner(cpu: &mut CpuRp2a03, operand: u8) {
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

    fn sbc_inner(cpu: &mut CpuRp2a03, operand: u8) {
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

    fn cmp_inner(cpu: &mut CpuRp2a03, reg: u8, mem: u8) {
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

    pub fn jmp_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        cpu.set_pc(addr_modes::abs(cpu, bus));
        3
    }

    pub fn jmp_ind(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let ptr = addr_modes::abs(cpu, bus);
        let lo = bus.read(ptr) as u16;
        // 6502 bug: when ptr ends in $FF, high byte is read from same page
        let hi_ptr = if ptr & 0xFF == 0xFF {
            ptr & 0xFF00
        } else {
            ptr.wrapping_add(1)
        };
        let hi = bus.read(hi_ptr) as u16;
        cpu.set_pc(lo | (hi << 8));
        5
    }

    pub fn jsr(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let target = addr_modes::abs(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let return_addr = cpu.pc().wrapping_sub(1);
        push(cpu, bus, (return_addr >> 8) as u8);
        push(cpu, bus, return_addr as u8);
        cpu.set_pc(target);
        6
    }

    pub fn rts(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let lo = pull(cpu, bus) as u16;
        let hi = pull(cpu, bus) as u16;
        cpu.set_pc((lo | (hi << 8)).wrapping_add(1));
        6
    }

    pub fn brk(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        cpu.set_pc(cpu.pc().wrapping_add(1));
        push(cpu, bus, (cpu.pc() >> 8) as u8);
        push(cpu, bus, cpu.pc() as u8);
        cpu.set_flag(FLAG_BREAK, true);
        push(cpu, bus, cpu.sr());
        cpu.set_flag(FLAG_INTERRUPT, true);
        cpu.set_pc(u16::from_le_bytes([bus.read(0xFFFE), bus.read(0xFFFF)]));
        7
    }

    pub fn rti(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let sr = pull(cpu, bus);
        cpu.set_sr(sr);
        let lo = pull(cpu, bus) as u16;
        let hi = pull(cpu, bus) as u16;
        cpu.set_pc(lo | (hi << 8));
        6
    }

    pub fn pha(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        push(cpu, bus, cpu.a());
        3
    }

    pub fn pla(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let val = pull(cpu, bus);
        cpu.set_a(val);
        cpu.set_sign(val);
        cpu.set_zero(val);
        4
    }

    pub fn php(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        push(cpu, bus, cpu.sr() | FLAG_BREAK);
        3
    }

    pub fn plp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let sr = pull(cpu, bus) & !FLAG_BREAK;
        cpu.set_sr(sr);
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

    pub fn clc(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
        cpu.set_flag(FLAG_CARRY, false);
        2
    }

    pub fn sec(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
        cpu.set_flag(FLAG_CARRY, true);
        2
    }

    pub fn cld(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
        cpu.set_flag(FLAG_DECIMAL, false);
        2
    }

    pub fn sed(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
        cpu.set_flag(FLAG_DECIMAL, true);
        2
    }

    pub fn cli(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
        cpu.set_flag(FLAG_INTERRUPT, false);
        2
    }

    pub fn sei(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
        cpu.set_flag(FLAG_INTERRUPT, true);
        2
    }

    pub fn clv(cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
        cpu.set_flag(FLAG_OVERFLOW, false);
        2
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

    // ---- SLO (ASL memory then ORA with A) ----
    pub fn slo_indx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::indx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let shifted = val << 1;
        bus.write(addr, shifted);
        let result = shifted | cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        8
    }

    pub fn slo_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zp(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let shifted = val << 1;
        bus.write(addr, shifted);
        let result = shifted | cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        5
    }

    pub fn slo_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::abs(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr);
        cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let shifted = val << 1;
        bus.write(addr, shifted);
        let result = shifted | cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        6
    }

    pub fn slo_indy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, page) = addr_modes::indy(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let shifted = val << 1;
        bus.write(addr, shifted);
        let result = shifted | cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        8 + page
    }

    pub fn slo_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zpx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let shifted = val << 1;
        bus.write(addr, shifted);
        let result = shifted | cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        6
    }

    pub fn slo_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, _) = addr_modes::absy(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr);
        cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let shifted = val << 1;
        bus.write(addr, shifted);
        let result = shifted | cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        7
    }

    pub fn slo_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, _) = addr_modes::absx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr);
        cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let shifted = val << 1;
        bus.write(addr, shifted);
        let result = shifted | cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        7
    }

    // ---- RLA (ROL memory then AND with A) ----
    pub fn rla_indx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::indx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        let carry = cpu.get_flag(FLAG_CARRY) as u8;
        cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let shifted = (val << 1) | carry;
        bus.write(addr, shifted);
        let result = shifted & cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        8
    }

    pub fn rla_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zp(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        let carry = cpu.get_flag(FLAG_CARRY) as u8;
        cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let shifted = (val << 1) | carry;
        bus.write(addr, shifted);
        let result = shifted & cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        5
    }

    pub fn rla_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::abs(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr);
        let carry = cpu.get_flag(FLAG_CARRY) as u8;
        cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let shifted = (val << 1) | carry;
        bus.write(addr, shifted);
        let result = shifted & cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        6
    }

    pub fn rla_indy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, page) = addr_modes::indy(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        let carry = cpu.get_flag(FLAG_CARRY) as u8;
        cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let shifted = (val << 1) | carry;
        bus.write(addr, shifted);
        let result = shifted & cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        8 + page
    }

    pub fn rla_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zpx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        let carry = cpu.get_flag(FLAG_CARRY) as u8;
        cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let shifted = (val << 1) | carry;
        bus.write(addr, shifted);
        let result = shifted & cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        6
    }

    pub fn rla_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, _) = addr_modes::absy(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr);
        let carry = cpu.get_flag(FLAG_CARRY) as u8;
        cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let shifted = (val << 1) | carry;
        bus.write(addr, shifted);
        let result = shifted & cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        7
    }

    pub fn rla_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, _) = addr_modes::absx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr);
        let carry = cpu.get_flag(FLAG_CARRY) as u8;
        cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
        let shifted = (val << 1) | carry;
        bus.write(addr, shifted);
        let result = shifted & cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        7
    }

    // ---- SRE (LSR memory then EOR with A) ----
    pub fn sre_indx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::indx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let shifted = val >> 1;
        bus.write(addr, shifted);
        let result = shifted ^ cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        8
    }

    pub fn sre_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zp(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let shifted = val >> 1;
        bus.write(addr, shifted);
        let result = shifted ^ cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        5
    }

    pub fn sre_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::abs(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr);
        cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let shifted = val >> 1;
        bus.write(addr, shifted);
        let result = shifted ^ cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        6
    }

    pub fn sre_indy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, page) = addr_modes::indy(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let shifted = val >> 1;
        bus.write(addr, shifted);
        let result = shifted ^ cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        8 + page
    }

    pub fn sre_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zpx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let shifted = val >> 1;
        bus.write(addr, shifted);
        let result = shifted ^ cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        6
    }

    pub fn sre_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, _) = addr_modes::absy(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr);
        cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let shifted = val >> 1;
        bus.write(addr, shifted);
        let result = shifted ^ cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        7
    }

    pub fn sre_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, _) = addr_modes::absx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr);
        cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let shifted = val >> 1;
        bus.write(addr, shifted);
        let result = shifted ^ cpu.a();
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        7
    }

    // ---- RRA (ROR memory then ADC with A) ----
    pub fn rra_indx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::indx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        let carry = cpu.get_flag(FLAG_CARRY) as u8;
        cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let rotated = (val >> 1) | (carry << 7);
        bus.write(addr, rotated);
        adc_inner(cpu, rotated);
        8
    }

    pub fn rra_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zp(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        let carry = cpu.get_flag(FLAG_CARRY) as u8;
        cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let rotated = (val >> 1) | (carry << 7);
        bus.write(addr, rotated);
        adc_inner(cpu, rotated);
        5
    }

    pub fn rra_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::abs(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr);
        let carry = cpu.get_flag(FLAG_CARRY) as u8;
        cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let rotated = (val >> 1) | (carry << 7);
        bus.write(addr, rotated);
        adc_inner(cpu, rotated);
        6
    }

    pub fn rra_indy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, page) = addr_modes::indy(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        let carry = cpu.get_flag(FLAG_CARRY) as u8;
        cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let rotated = (val >> 1) | (carry << 7);
        bus.write(addr, rotated);
        adc_inner(cpu, rotated);
        8 + page
    }

    pub fn rra_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zpx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        let carry = cpu.get_flag(FLAG_CARRY) as u8;
        cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let rotated = (val >> 1) | (carry << 7);
        bus.write(addr, rotated);
        adc_inner(cpu, rotated);
        6
    }

    pub fn rra_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, _) = addr_modes::absy(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr);
        let carry = cpu.get_flag(FLAG_CARRY) as u8;
        cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let rotated = (val >> 1) | (carry << 7);
        bus.write(addr, rotated);
        adc_inner(cpu, rotated);
        7
    }

    pub fn rra_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, _) = addr_modes::absx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr);
        let carry = cpu.get_flag(FLAG_CARRY) as u8;
        cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let rotated = (val >> 1) | (carry << 7);
        bus.write(addr, rotated);
        adc_inner(cpu, rotated);
        7
    }

    // ---- SAX (Store A & X at address) ----
    pub fn sax_indx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::indx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        bus.write(addr, cpu.a() & cpu.x());
        6
    }

    pub fn sax_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zp(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        bus.write(addr, cpu.a() & cpu.x());
        3
    }

    pub fn sax_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::abs(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        bus.write(addr, cpu.a() & cpu.x());
        4
    }

    pub fn sax_zpy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zpy(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        bus.write(addr, cpu.a() & cpu.x());
        4
    }

    // ---- LAX (Load A and X from address) ----
    pub fn lax_indx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::indx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        cpu.set_a(val);
        cpu.set_x(val);
        cpu.set_sign(val);
        cpu.set_zero(val);
        6
    }

    pub fn lax_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zp(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        cpu.set_a(val);
        cpu.set_x(val);
        cpu.set_sign(val);
        cpu.set_zero(val);
        3
    }

    pub fn lax_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::abs(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr);
        cpu.set_a(val);
        cpu.set_x(val);
        cpu.set_sign(val);
        cpu.set_zero(val);
        4
    }

    pub fn lax_indy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, page) = addr_modes::indy(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        cpu.set_a(val);
        cpu.set_x(val);
        cpu.set_sign(val);
        cpu.set_zero(val);
        5 + page
    }

    pub fn lax_zpy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zpy(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr);
        cpu.set_a(val);
        cpu.set_x(val);
        cpu.set_sign(val);
        cpu.set_zero(val);
        4
    }

    pub fn lax_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, page) = addr_modes::absy(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr);
        cpu.set_a(val);
        cpu.set_x(val);
        cpu.set_sign(val);
        cpu.set_zero(val);
        4 + page
    }

    // ---- DCP (DEC memory then CMP with A) ----
    pub fn dcp_indx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::indx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr).wrapping_sub(1);
        bus.write(addr, val);
        cmp_inner(cpu, cpu.a(), val);
        8
    }

    pub fn dcp_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zp(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr).wrapping_sub(1);
        bus.write(addr, val);
        cmp_inner(cpu, cpu.a(), val);
        5
    }

    pub fn dcp_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::abs(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr).wrapping_sub(1);
        bus.write(addr, val);
        cmp_inner(cpu, cpu.a(), val);
        6
    }

    pub fn dcp_indy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, page) = addr_modes::indy(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr).wrapping_sub(1);
        bus.write(addr, val);
        cmp_inner(cpu, cpu.a(), val);
        8 + page
    }

    pub fn dcp_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zpx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr).wrapping_sub(1);
        bus.write(addr, val);
        cmp_inner(cpu, cpu.a(), val);
        6
    }

    pub fn dcp_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, _) = addr_modes::absy(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr).wrapping_sub(1);
        bus.write(addr, val);
        cmp_inner(cpu, cpu.a(), val);
        7
    }

    pub fn dcp_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, _) = addr_modes::absx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr).wrapping_sub(1);
        bus.write(addr, val);
        cmp_inner(cpu, cpu.a(), val);
        7
    }

    // ---- ISC (INC memory then SBC with A) ----
    pub fn isc_indx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::indx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr).wrapping_add(1);
        bus.write(addr, val);
        sbc_inner(cpu, val);
        8
    }

    pub fn isc_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zp(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr).wrapping_add(1);
        bus.write(addr, val);
        sbc_inner(cpu, val);
        5
    }

    pub fn isc_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::abs(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr).wrapping_add(1);
        bus.write(addr, val);
        sbc_inner(cpu, val);
        6
    }

    pub fn isc_indy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, page) = addr_modes::indy(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr).wrapping_add(1);
        bus.write(addr, val);
        sbc_inner(cpu, val);
        8 + page
    }

    pub fn isc_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zpx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = bus.read(addr).wrapping_add(1);
        bus.write(addr, val);
        sbc_inner(cpu, val);
        6
    }

    pub fn isc_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, _) = addr_modes::absy(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr).wrapping_add(1);
        bus.write(addr, val);
        sbc_inner(cpu, val);
        7
    }

    pub fn isc_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, _) = addr_modes::absx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let val = bus.read(addr).wrapping_add(1);
        bus.write(addr, val);
        sbc_inner(cpu, val);
        7
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

    // ---- ASR (AND with A, then LSR A) ----
    pub fn asr_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let val = bus.read(cpu.pc()) & cpu.a();
        cpu.set_pc(cpu.pc().wrapping_add(1));
        cpu.set_flag(FLAG_CARRY, val & 0x01 != 0);
        let result = val >> 1;
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        2
    }

    // ---- ARR (AND with A, then ROR A with unusual flags) ----
    pub fn arr_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let val = bus.read(cpu.pc()) & cpu.a();
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let old_carry = cpu.get_flag(FLAG_CARRY) as u8;
        let result = (val >> 1) | (old_carry << 7);
        cpu.set_a(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        cpu.set_flag(FLAG_CARRY, val & 0x80 != 0);
        cpu.set_flag(FLAG_OVERFLOW, (((val >> 5) ^ (val >> 6)) & 1) != 0);
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

    // ---- AXS (X = (A & X) - operand) ----
    pub fn axs_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let operand = bus.read(cpu.pc());
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let val = cpu.a() & cpu.x();
        let result = val.wrapping_sub(operand);
        cpu.set_x(result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        cpu.set_flag(FLAG_CARRY, val >= operand);
        2
    }

    // ---- SHA (Store A & X & H at address) ----
    // Behavior 2: high byte ANDed with X only on page cross.
    // If a DMC DMA just fired between instructions, ignore H.
    pub fn sha_indy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let ptr = bus.read(cpu.pc()) as u16;
        let base = bus.read(ptr) as u16 | (bus.read(ptr.wrapping_add(1)) as u16) << 8;
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let addr = base.wrapping_add(cpu.y() as u16);
        let page = (((base ^ addr) >> 8) as u8) & 1;
        let h = ((base >> 8) as u8).wrapping_add(1);
        let ignore_h = core::mem::replace(&mut bus.dmc_just_fired, false);
        let final_h = if ignore_h { 0xFF } else { h };
        let val = cpu.a() & cpu.x() & final_h;
        let _ = bus.read(addr);
        let final_addr = if page != 0 {
            (addr as u8 as u16) | ((h & cpu.x()) as u16) << 8
        } else {
            addr
        };
        bus.write(final_addr, val);
        6
    }

    pub fn sha_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let lo = bus.read(cpu.pc()) as u16;
        let hi = bus.read(cpu.pc().wrapping_add(1)) as u16;
        let base = lo | (hi << 8);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let addr = base.wrapping_add(cpu.y() as u16);
        let page = (((base ^ addr) >> 8) as u8) & 1;
        let h = ((base >> 8) as u8).wrapping_add(1);
        let ignore_h = core::mem::replace(&mut bus.dmc_just_fired, false);
        let final_h = if ignore_h { 0xFF } else { h };
        let val = cpu.a() & cpu.x() & final_h;
        let _ = bus.read(addr);
        let final_addr = if page != 0 {
            (addr as u8 as u16) | ((h & cpu.x()) as u16) << 8
        } else {
            addr
        };
        bus.write(final_addr, val);
        5
    }

    // ---- SHS (Same as SHA abs,y but also sets SP = A & X) ----
    pub fn shs_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let lo = bus.read(cpu.pc()) as u16;
        let hi = bus.read(cpu.pc().wrapping_add(1)) as u16;
        let base = lo | (hi << 8);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let addr = base.wrapping_add(cpu.y() as u16);
        let page = (((base ^ addr) >> 8) as u8) & 1;
        let h = ((base >> 8) as u8).wrapping_add(1);
        let sp_val = cpu.a() & cpu.x();
        cpu.set_st(sp_val);
        let ignore_h = core::mem::replace(&mut bus.dmc_just_fired, false);
        let final_h = if ignore_h { 0xFF } else { h };
        let val = sp_val & final_h;
        let _ = bus.read(addr);
        let final_addr = if page != 0 {
            (addr as u8 as u16) | ((h & cpu.x()) as u16) << 8
        } else {
            addr
        };
        bus.write(final_addr, val);
        5
    }

    // ---- SHY (Store Y & H at address) ----
    pub fn shy_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let lo = bus.read(cpu.pc()) as u16;
        let hi = bus.read(cpu.pc().wrapping_add(1)) as u16;
        let base = lo | (hi << 8);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let addr = base.wrapping_add(cpu.x() as u16);
        let page = (((base ^ addr) >> 8) as u8) & 1;
        let h = ((base >> 8) as u8).wrapping_add(1);
        let ignore_h = core::mem::replace(&mut bus.dmc_just_fired, false);
        let final_h = if ignore_h { 0xFF } else { h };
        let val = cpu.y() & final_h;
        let _ = bus.read(addr);
        let final_addr = if page != 0 {
            (addr as u8 as u16) | ((h & cpu.y()) as u16) << 8
        } else {
            addr
        };
        bus.write(final_addr, val);
        5
    }

    // ---- SHX (Store X & (H+1) at address) ----
    pub fn shx_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let lo = bus.read(cpu.pc()) as u16;
        let hi = bus.read(cpu.pc().wrapping_add(1)) as u16;
        let base = lo | (hi << 8);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let addr = base.wrapping_add(cpu.y() as u16);
        let page = (((base ^ addr) >> 8) as u8) & 1;
        let h = ((base >> 8) as u8).wrapping_add(1);
        let ignore_h = core::mem::replace(&mut bus.dmc_just_fired, false);
        let final_h = if ignore_h { 0xFF } else { h };
        let val = cpu.x() & final_h;
        let _ = bus.read(addr);
        let final_addr = if page != 0 {
            (addr as u8 as u16) | ((h & cpu.x()) as u16) << 8
        } else {
            addr
        };
        bus.write(final_addr, val);
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

    // ---- NOP variants ----
    pub fn nop_imm(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        bus.read(cpu.pc());
        cpu.set_pc(cpu.pc().wrapping_add(1));
        2
    }

    pub fn nop_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zp(cpu, bus);
        // Dummy read from the zero page address (cycle 3 of NOP zp)
        let _ = bus.read(addr);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        3
    }

    // ---- NOP Zero Page, X ----
    pub fn nop_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zpx(cpu, bus);
        // Dummy read from the ZPX address (cycle 4 of NOP zpx)
        let _ = bus.read(addr);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        4
    }

    // ---- NOP Absolute ----
    pub fn nop_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::abs(cpu, bus);
        // Dummy read from the absolute address (cycle 4 of NOP abs)
        let _ = bus.read(addr);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        4
    }

    // ---- NOP Absolute, X ----
    pub fn nop_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, page) = addr_modes::absx(cpu, bus);
        // Dummy read from the final address (cycle 4/5 of NOP absx)
        let _ = bus.read(addr);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        4 + page
    }

    pub fn nop(_cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
        2
    }

    #[allow(dead_code)]
    pub fn illegal(_cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
        2
    }
}
