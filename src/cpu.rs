#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct Cpu6502 {
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub st: u8,
    pub status: u8,
}

impl Cpu6502 {
    pub fn new(reset_addr: u16) -> Self {
        Self {
            pc: reset_addr,
            ..Default::default()
        }
    }

    pub fn as_bytes(&self) -> &[u8; 7] {
        unsafe { &*(self as *const Self as *const [u8; 7]) }
    }

    pub fn from_bytes(bytes: &[u8; 7]) -> Self {
        unsafe { *(bytes as *const [u8; 7] as *const Self) }
    }
}

pub const FLAG_CARRY: u8 = 0b0000_0001;
pub const FLAG_ZERO: u8 = 0b0000_0010;
pub const FLAG_INTERRUPT: u8 = 0b0000_0100;
pub const FLAG_DECIMAL: u8 = 0b0000_1000;
pub const FLAG_BREAK: u8 = 0b0001_0000;
pub const FLAG_OVERFLOW: u8 = 0b0100_0000;
pub const FLAG_NEGATIVE: u8 = 0b1000_0000;
