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
        (addr, (((base ^ addr) >> 8) as u8) & 1)
    }

    #[inline(always)]
    pub fn absy(cpu: &CpuRp2a03, bus: &mut Bus) -> (u16, u8) {
        let base = abs(cpu, bus);
        let addr = base.wrapping_add(cpu.y() as u16);
        (addr, (((base ^ addr) >> 8) as u8) & 1)
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
        let hi = bus.read(ptr.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    #[inline(always)]
    pub fn indy(cpu: &CpuRp2a03, bus: &mut Bus) -> (u16, u8) {
        let ptr = bus.read(cpu.pc()) as u16;
        let base = bus.read(ptr) as u16 | (bus.read(ptr.wrapping_add(1)) as u16) << 8;
        let addr = base.wrapping_add(cpu.y() as u16);
        (addr, (((base ^ addr) >> 8) as u8) & 1)
    }
}

pub static TABLE: [Op; 256] = {
    use self::op::*;
    [
        brk, ora_indx, illegal, illegal, illegal, ora_zp, asl_zp, illegal, php, ora_imm, asl_a,
        illegal, illegal, ora_abs, asl_abs, illegal, bpl, ora_indy, illegal, illegal, illegal,
        ora_zpx, asl_zpx, illegal, clc, ora_absy, illegal, illegal, illegal, ora_absx, asl_absx,
        illegal, jsr, and_indx, illegal, illegal, bit_zp, and_zp, rol_zp, illegal, plp, and_imm,
        rol_a, illegal, bit_abs, and_abs, rol_abs, illegal, bmi, and_indy, illegal, illegal,
        illegal, and_zpx, rol_zpx, illegal, sec, and_absy, illegal, illegal, illegal, and_absx,
        rol_absx, illegal, rti, eor_indx, illegal, illegal, illegal, eor_zp, lsr_zp, illegal, pha,
        eor_imm, lsr_a, illegal, jmp_abs, eor_abs, lsr_abs, illegal, bvc, eor_indy, illegal,
        illegal, illegal, eor_zpx, lsr_zpx, illegal, cli, eor_absy, illegal, illegal, illegal,
        eor_absx, lsr_absx, illegal, rts, adc_indx, illegal, illegal, illegal, adc_zp, ror_zp,
        illegal, pla, adc_imm, ror_a, illegal, jmp_ind, adc_abs, ror_abs, illegal, bvs, adc_indy,
        illegal, illegal, illegal, adc_zpx, ror_zpx, illegal, sei, adc_absy, illegal, illegal,
        illegal, adc_absx, ror_absx, illegal, illegal, sta_indx, illegal, illegal, sty_zp, sta_zp,
        stx_zp, illegal, dey, illegal, txa, illegal, sty_abs, sta_abs, stx_abs, illegal, bcc,
        sta_indy, illegal, illegal, sty_zpx, sta_zpx, stx_zpy, illegal, tya, sta_absy, txs,
        illegal, illegal, sta_absx, illegal, illegal, ldy_imm, lda_indx, ldx_imm, illegal, ldy_zp,
        lda_zp, ldx_zp, illegal, tay, lda_imm, tax, illegal, ldy_abs, lda_abs, ldx_abs, illegal,
        bcs, lda_indy, illegal, illegal, ldy_zpx, lda_zpx, ldx_zpy, illegal, clv, lda_absy, tsx,
        illegal, ldy_absx, lda_absx, ldx_absy, illegal, cpy_imm, cmp_indx, illegal, illegal,
        cpy_zp, cmp_zp, dec_zp, illegal, iny, cmp_imm, dex, illegal, cpy_abs, cmp_abs, dec_abs,
        illegal, bne, cmp_indy, illegal, illegal, illegal, cmp_zpx, dec_zpx, illegal, cld,
        cmp_absy, illegal, illegal, illegal, cmp_absx, dec_absx, illegal, cpx_imm, sbc_indx,
        illegal, illegal, cpx_zp, sbc_zp, inc_zp, illegal, inx, sbc_imm, nop, illegal, cpx_abs,
        sbc_abs, inc_abs, illegal, beq, sbc_indy, illegal, illegal, illegal, sbc_zpx, inc_zpx,
        illegal, sed, sbc_absy, illegal, illegal, illegal, sbc_absx, inc_absx, illegal,
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
        let (addr, _) = addr_modes::absx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        bus.write(addr, cpu.a());
        5
    }

    pub fn sta_absy(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, _) = addr_modes::absy(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
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
        if cpu.get_flag(FLAG_DECIMAL) {
            let low = (a & 0x0F) + (operand & 0x0F) + cpu.get_flag(FLAG_CARRY) as u8;
            let mut temp = result;
            if low > 9 {
                temp += 6;
            }
            cpu.set_flag(FLAG_CARRY, temp > 0x99);
            if temp > 0x99 {
                temp += 96;
            }
            cpu.set_a(temp as u8);
            cpu.set_sign(cpu.a());
            cpu.set_zero(cpu.a());
            cpu.set_flag(
                FLAG_OVERFLOW,
                !((a ^ operand) & 0x80) != 0 && ((a ^ cpu.a()) & 0x80) != 0,
            );
        } else {
            cpu.set_sign(lo);
            cpu.set_zero(lo);
            cpu.set_flag(FLAG_CARRY, result > 0xFF);
            cpu.set_flag(
                FLAG_OVERFLOW,
                !((a ^ operand) & 0x80) != 0 && ((a ^ lo) & 0x80) != 0,
            );
            cpu.set_a(lo);
        }
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
        if cpu.get_flag(FLAG_DECIMAL) {
            let low = (a & 0x0F)
                .wrapping_sub(operand & 0x0F)
                .wrapping_sub(1 - cpu.get_flag(FLAG_CARRY) as u8);
            let mut temp = result;
            if low & 0x80 != 0 {
                temp = temp.wrapping_sub(6);
            }
            cpu.set_flag(FLAG_CARRY, temp < 0x100);
            if temp & 0xFF00 != 0 {
                temp = temp.wrapping_sub(0x60);
            }
            cpu.set_a(temp as u8);
            cpu.set_sign(cpu.a());
            cpu.set_zero(cpu.a());
            cpu.set_flag(
                FLAG_OVERFLOW,
                ((a ^ lo) & 0x80) != 0 && ((a ^ operand) & 0x80) != 0,
            );
        } else {
            cpu.set_sign(lo);
            cpu.set_zero(lo);
            cpu.set_flag(FLAG_CARRY, result < 0x100);
            cpu.set_flag(
                FLAG_OVERFLOW,
                ((a ^ lo) & 0x80) != 0 && ((a ^ operand) & 0x80) != 0,
            );
            cpu.set_a(lo);
        }
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
        let result = bus.read(addr).wrapping_add(1);
        bus.write(addr, result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        5
    }

    pub fn inc_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zpx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let result = bus.read(addr).wrapping_add(1);
        bus.write(addr, result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        6
    }

    pub fn inc_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::abs(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let result = bus.read(addr).wrapping_add(1);
        bus.write(addr, result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        6
    }

    pub fn inc_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, _) = addr_modes::absx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let result = bus.read(addr).wrapping_add(1);
        bus.write(addr, result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        7
    }

    pub fn dec_zp(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zp(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let result = bus.read(addr).wrapping_sub(1);
        bus.write(addr, result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        5
    }

    pub fn dec_zpx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::zpx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(1));
        let result = bus.read(addr).wrapping_sub(1);
        bus.write(addr, result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        6
    }

    pub fn dec_abs(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let addr = addr_modes::abs(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let result = bus.read(addr).wrapping_sub(1);
        bus.write(addr, result);
        cpu.set_sign(result);
        cpu.set_zero(result);
        6
    }

    pub fn dec_absx(cpu: &mut CpuRp2a03, bus: &mut Bus) -> u8 {
        let (addr, _) = addr_modes::absx(cpu, bus);
        cpu.set_pc(cpu.pc().wrapping_add(2));
        let result = bus.read(addr).wrapping_sub(1);
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
        let hi = bus.read(ptr.wrapping_add(1)) as u16;
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

    pub fn nop(_cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
        2
    }

    pub fn illegal(_cpu: &mut CpuRp2a03, _bus: &mut Bus) -> u8 {
        2
    }
}
