pub fn is_ram(addr: u16) -> bool {
    addr & 0xE000 == 0x0000
}

pub fn is_ppu(addr: u16) -> bool {
    addr & 0xE000 == 0x2000
}

pub fn is_apu_io(addr: u16) -> bool {
    addr & 0xFFE0 == 0x4000
}

pub fn is_cartridge(addr: u16) -> bool {
    addr >= 0x4020
}
