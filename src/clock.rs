pub fn master_cycles(cpu_cycles: u8) -> u32 {
    cpu_cycles as u32 * 12
}

pub fn ppu_cycles(cpu_cycles: u8) -> u32 {
    cpu_cycles as u32 * 3
}
