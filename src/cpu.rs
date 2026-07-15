pub const FLAG_CARRY: u8     = 0b0000_0001;
pub const FLAG_ZERO: u8      = 0b0000_0010;
pub const FLAG_INTERRUPT: u8 = 0b0000_0100;
pub const FLAG_DECIMAL: u8   = 0b0000_1000;
pub const FLAG_BREAK: u8     = 0b0001_0000;
pub const FLAG_OVERFLOW: u8  = 0b0100_0000;
pub const FLAG_NEGATIVE: u8  = 0b1000_0000;

#[repr(C, align(2))]
#[derive(Clone, Copy)]
pub struct CpuRp2a03 {
    // Layout:
    // 0..2  => PC (u16 - Program Counter)
    // 2     => A  (u8  - Accumulator)
    // 3     => X  (u8  - Index Register X)
    // 4     => Y  (u8  - Index Register Y)
    // 5     => ST (u8  - Stack Pointer)
    // 6     => SR (u8  - Status Register 'P')
    bytes: [u8; 7],
}

impl CpuRp2a03 {
    pub fn new(reset_addr: u16) -> Self {
        let mut cpu = Self::default();
        cpu.set_pc(reset_addr);
        cpu
    }

    #[inline(always)]
    pub fn pc(&self) -> u16 {
        u16::from_le_bytes([self.bytes[0], self.bytes[1]])
    }

    #[inline(always)]
    pub fn set_pc(&mut self, val: u16) {
        let le = val.to_le_bytes();
        self.bytes[0] = le[0];
        self.bytes[1] = le[1];
    }
    
    #[inline(always)]
    pub fn advance_pc(&mut self, n: u16) {
        let next = self.pc().wrapping_add(n).to_le_bytes();
        self.bytes[0] = next[0];
        self.bytes[1] = next[1];
    }

    #[inline(always)]
    pub fn a(&self) -> u8 { self.bytes[2] }

    #[inline(always)]
    pub fn set_a(&mut self, val: u8) { self.bytes[2] = val; }

    #[inline(always)]
    pub fn x(&self) -> u8 { self.bytes[3] }

    #[inline(always)]
    pub fn set_x(&mut self, val: u8) { self.bytes[3] = val; }

    #[inline(always)]
    pub fn y(&self) -> u8 { self.bytes[4] }

    #[inline(always)]
    pub fn set_y(&mut self, val: u8) { self.bytes[4] = val; }

    #[inline(always)]
    pub fn st(&self) -> u8 { self.bytes[5] }

    #[inline(always)]
    pub fn set_st(&mut self, val: u8) { self.bytes[5] = val; }

    #[inline(always)]
    pub fn sr(&self) -> u8 { self.bytes[6] }

    #[inline(always)]
    pub fn set_sr(&mut self, val: u8) { self.bytes[6] = val; }

    #[inline(always)]
    pub fn get_flag(&self, flag: u8) -> bool {
        (self.bytes[6] & flag) != 0
    }

    #[inline(always)]
    pub fn set_flag(&mut self, flag: u8, set: bool) {
        self.bytes[6] = (self.bytes[6] & !flag) | (flag & (set as u8).wrapping_neg());
    }

    #[inline(always)]
    pub fn set_sign(&mut self, val: u8) {
        self.set_flag(FLAG_NEGATIVE, (val & 0x80) != 0);
    }

    #[inline(always)]
    pub fn set_zero(&mut self, val: u8) {
        self.set_flag(FLAG_ZERO, val == 0);
    }

    #[inline(always)]
    pub fn update_zn_flags(&mut self, val: u8) {
        self.bytes[6] &= !(FLAG_NEGATIVE | FLAG_ZERO);
        if val == 0 { self.bytes[6] |= FLAG_ZERO; }
        if (val & 0x80) != 0 { self.bytes[6] |= FLAG_NEGATIVE; }
    }

    pub fn as_bytes(&self) -> &[u8; 7] {
        &self.bytes
    }

    pub fn from_bytes(bytes: &[u8; 7]) -> Self {
        Self { bytes: *bytes }
    }
}

impl Default for CpuRp2a03 {
    fn default() -> Self {
        Self { bytes: [0; 7] }
    }
}
