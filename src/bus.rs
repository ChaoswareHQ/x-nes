use crate::apu::Apu;
use crate::gamepad::Gamepad;
use crate::ppu::Ppu;

pub struct Bus<'a> {
    pub ram: [u8; 2048],
    pub prg: &'a [u8],
    pub ppu: Ppu,
    pub apu: Apu,
    pub pad1: Gamepad,
}

impl<'a> Bus<'a> {
    pub fn new(prg: &'a [u8], chr: &[u8]) -> Self {
        Self {
            ram: [0; 2048],
            prg,
            ppu: Ppu::new(chr),
            apu: Apu::new(),
            pad1: Gamepad::new(),
        }
    }

    #[inline(always)]
    pub fn read(&mut self, addr: u16) -> u8 {
        let top = (addr >> 12) as u8;
        match top {
            0 | 1 => self.ram[(addr & 0x07FF) as usize],
            2 | 3 => self.read_ppu(addr),
            4 if addr < 0x4020 => match addr {
                0x4016 => self.pad1.read(),
                _ => self.apu.read(addr),
            },
            _ => {
                if addr < 0x8000 || self.prg.is_empty() {
                    0
                } else {
                    self.prg[((addr - 0x8000) as usize) % self.prg.len()]
                }
            }
        }
    }

    #[inline(always)]
    pub fn write(&mut self, addr: u16, val: u8) {
        let top = (addr >> 12) as u8;
        match top {
            0 | 1 => self.ram[(addr & 0x07FF) as usize] = val,
            2 | 3 => self.write_ppu(addr, val),
            4 if addr < 0x4020 => match addr {
                0x4014 => self.oam_dma(val),
                0x4016 => {
                    self.pad1.write(val);
                    self.apu.write(addr, val);
                }
                _ => self.apu.write(addr, val),
            },
            _ => {}
        }
    }

    #[inline(always)]
    fn read_ppu(&mut self, addr: u16) -> u8 {
        match addr & 7 {
            2 => self.ppu.read_status(),
            4 => self.ppu.read_oam_data(),
            7 => self.ppu.read_data(),
            _ => 0,
        }
    }

    #[inline(always)]
    fn write_ppu(&mut self, addr: u16, val: u8) {
        match addr & 7 {
            0 => self.ppu.write_ctrl(val),
            1 => self.ppu.write_mask(val),
            3 => self.ppu.write_oam_addr(val),
            4 => self.ppu.write_oam_data(val),
            5 => self.ppu.write_scroll(val),
            6 => self.ppu.write_addr(val),
            7 => self.ppu.write_data(val),
            _ => {}
        }
    }

    fn oam_dma(&mut self, page: u8) {
        let base = (page as u16) << 8;
        for i in 0..256 {
            let addr = base | i;
            let val = self.read(addr);
            self.ppu.oam[self.ppu.oam_addr as usize] = val;
            self.ppu.oam_addr = self.ppu.oam_addr.wrapping_add(1);
        }
    }

    pub fn poll_nmi(&mut self) -> bool {
        if self.ppu.nmi_pending {
            self.ppu.nmi_pending = false;
            true
        } else {
            false
        }
    }
}
