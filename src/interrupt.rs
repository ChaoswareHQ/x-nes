pub const NMI_VECTOR: u16 = 0xFFFA;
pub const RESET_VECTOR: u16 = 0xFFFC;
pub const IRQ_VECTOR: u16 = 0xFFFE;

pub fn read_vector(bus: &impl Fn(u16) -> u8, addr: u16) -> u16 {
    let lo = bus(addr) as u16;
    let hi = bus(addr.wrapping_add(1)) as u16;
    lo | (hi << 8)
}
