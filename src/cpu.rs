#[repr(C)]
#[derive(Clone, Copy)]
pub struct Cpu6502 {
    bytes: [u8; 7],
}

impl Cpu6502 {
    pub fn new(reset_addr: u16) -> Self {
        let mut cpu = Self { bytes: [0; 7] };
        cpu.set_pc(reset_addr);
        cpu
    }

    #[inline]
    pub fn pc(&self) -> u16 {
        u16::from_le_bytes([self.bytes[0], self.bytes[1]])
    }
    #[inline]
    pub fn set_pc(&mut self, val: u16) {
        self.bytes[0..2].copy_from_slice(&val.to_le_bytes());
    }

    #[inline]
    pub fn a(&self) -> u8 {
        self.bytes[2]
    }
    #[inline]
    pub fn set_a(&mut self, val: u8) {
        self.bytes[2] = val;
    }

    #[inline]
    pub fn x(&self) -> u8 {
        self.bytes[3]
    }
    #[inline]
    pub fn set_x(&mut self, val: u8) {
        self.bytes[3] = val;
    }

    #[inline]
    pub fn y(&self) -> u8 {
        self.bytes[4]
    }
    #[inline]
    pub fn set_y(&mut self, val: u8) {
        self.bytes[4] = val;
    }

    #[inline]
    pub fn st(&self) -> u8 {
        self.bytes[5]
    }
    #[inline]
    pub fn set_st(&mut self, val: u8) {
        self.bytes[5] = val;
    }

    #[inline]
    pub fn status(&self) -> u8 {
        self.bytes[6]
    }
    #[inline]
    pub fn set_status(&mut self, val: u8) {
        self.bytes[6] = val;
    }

    #[inline]
    pub fn set_flag(&mut self, flag: u8, set: bool) {
        if set {
            self.bytes[6] |= flag;
        } else {
            self.bytes[6] &= !flag;
        }
    }

    #[inline]
    pub fn get_flag(&self, flag: u8) -> bool {
        self.bytes[6] & flag != 0
    }
}

impl Cpu6502 {
    pub fn as_bytes(&self) -> &[u8; 7] {
        &self.bytes
    }

    pub fn from_bytes(bytes: &[u8; 7]) -> Self {
        Self { bytes: *bytes }
    }
}

impl Default for Cpu6502 {
    fn default() -> Self {
        Self { bytes: [0; 7] }
    }
}

pub const FLAG_CARRY: u8 = 0b0000_0001;
pub const FLAG_ZERO: u8 = 0b0000_0010;
pub const FLAG_INTERRUPT: u8 = 0b0000_0100;
pub const FLAG_DECIMAL: u8 = 0b0000_1000;
pub const FLAG_BREAK: u8 = 0b0001_0000;
pub const FLAG_OVERFLOW: u8 = 0b0100_0000;
pub const FLAG_NEGATIVE: u8 = 0b1000_0000;
