use alloc::boxed::Box;

pub mod axrom;
pub mod cnrom;
pub mod gxrom;
pub mod mmc1;
pub mod mmc3;
pub mod nrom;
pub mod uxrom;

pub trait MapperImpl {
    fn cpu_read(&mut self, addr: u16) -> u8;
    fn cpu_write(&mut self, addr: u16, val: u8);
    fn ppu_read(&mut self, addr: u16) -> u8;
    fn ppu_write(&mut self, addr: u16, val: u8);
    fn mirroring(&self) -> u8;
    fn irq_pending(&self) -> bool;
    fn ack_irq(&mut self);
    fn clock_scanline(&mut self) {}

    fn has_chr_ram(&self) -> bool;
}

pub enum Mapper {
    Nrom(Box<nrom::Nrom>),
    UxRom(Box<uxrom::UxRom>),
    Cnrom(Box<cnrom::Cnrom>),
    Mmc1(Box<mmc1::Mmc1>),
    Mmc3(Box<mmc3::Mmc3>),
    Axrom(Box<axrom::Axrom>),
    Gxrom(Box<gxrom::Gxrom>),
    Null,
}

impl Mapper {
    pub fn from_ines(
        id: u8,
        mirroring: u8,
        prg_data: &[u8],
        chr_data: &[u8],
        chr_ram: bool,
    ) -> Self {
        match id {
            1 => Self::Mmc1(Box::new(mmc1::Mmc1::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            2 => Self::UxRom(Box::new(uxrom::UxRom::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            3 => Self::Cnrom(Box::new(cnrom::Cnrom::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            4 => Self::Mmc3(Box::new(mmc3::Mmc3::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            7 => Self::Axrom(Box::new(axrom::Axrom::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            66 => Self::Gxrom(Box::new(gxrom::Gxrom::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            _ => Self::Nrom(Box::new(nrom::Nrom::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
        }
    }

    #[inline(always)]
    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        match self {
            Self::Nrom(m) => m.cpu_read(addr),
            Self::UxRom(m) => m.cpu_read(addr),
            Self::Cnrom(m) => m.cpu_read(addr),
            Self::Mmc1(m) => m.cpu_read(addr),
            Self::Mmc3(m) => m.cpu_read(addr),
            Self::Axrom(m) => m.cpu_read(addr),
            Self::Gxrom(m) => m.cpu_read(addr),
            Self::Null => 0,
        }
    }

    #[inline(always)]
    pub fn cpu_write(&mut self, addr: u16, val: u8) {
        match self {
            Self::Nrom(m) => m.cpu_write(addr, val),
            Self::UxRom(m) => m.cpu_write(addr, val),
            Self::Cnrom(m) => m.cpu_write(addr, val),
            Self::Mmc1(m) => m.cpu_write(addr, val),
            Self::Mmc3(m) => m.cpu_write(addr, val),
            Self::Axrom(m) => m.cpu_write(addr, val),
            Self::Gxrom(m) => m.cpu_write(addr, val),
            Self::Null => {}
        }
    }

    #[inline(always)]
    pub fn ppu_read(&mut self, addr: u16) -> u8 {
        match self {
            Self::Nrom(m) => m.ppu_read(addr),
            Self::UxRom(m) => m.ppu_read(addr),
            Self::Cnrom(m) => m.ppu_read(addr),
            Self::Mmc1(m) => m.ppu_read(addr),
            Self::Mmc3(m) => m.ppu_read(addr),
            Self::Axrom(m) => m.ppu_read(addr),
            Self::Gxrom(m) => m.ppu_read(addr),
            Self::Null => 0,
        }
    }

    #[inline(always)]
    pub fn ppu_write(&mut self, addr: u16, val: u8) {
        match self {
            Self::Nrom(m) => m.ppu_write(addr, val),
            Self::UxRom(m) => m.ppu_write(addr, val),
            Self::Cnrom(m) => m.ppu_write(addr, val),
            Self::Mmc1(m) => m.ppu_write(addr, val),
            Self::Mmc3(m) => m.ppu_write(addr, val),
            Self::Axrom(m) => m.ppu_write(addr, val),
            Self::Gxrom(m) => m.ppu_write(addr, val),
            Self::Null => {}
        }
    }

    pub fn mirroring(&self) -> u8 {
        match self {
            Self::Nrom(m) => m.mirroring(),
            Self::UxRom(m) => m.mirroring(),
            Self::Cnrom(m) => m.mirroring(),
            Self::Mmc1(m) => m.mirroring(),
            Self::Mmc3(m) => m.mirroring(),
            Self::Axrom(m) => m.mirroring(),
            Self::Gxrom(m) => m.mirroring(),
            Self::Null => 0,
        }
    }

    pub fn irq_pending(&self) -> bool {
        match self {
            Self::Nrom(m) => m.irq_pending(),
            Self::UxRom(m) => m.irq_pending(),
            Self::Cnrom(m) => m.irq_pending(),
            Self::Mmc1(m) => m.irq_pending(),
            Self::Mmc3(m) => m.irq_pending(),
            Self::Axrom(m) => m.irq_pending(),
            Self::Gxrom(m) => m.irq_pending(),
            Self::Null => false,
        }
    }

    pub fn ack_irq(&mut self) {
        match self {
            Self::Nrom(m) => m.ack_irq(),
            Self::UxRom(m) => m.ack_irq(),
            Self::Cnrom(m) => m.ack_irq(),
            Self::Mmc1(m) => m.ack_irq(),
            Self::Mmc3(m) => m.ack_irq(),
            Self::Axrom(m) => m.ack_irq(),
            Self::Gxrom(m) => m.ack_irq(),
            Self::Null => {}
        }
    }

    pub fn clock_scanline(&mut self) {
        match self {
            Self::Nrom(m) => m.clock_scanline(),
            Self::UxRom(m) => m.clock_scanline(),
            Self::Cnrom(m) => m.clock_scanline(),
            Self::Mmc1(m) => m.clock_scanline(),
            Self::Mmc3(m) => m.clock_scanline(),
            Self::Axrom(m) => m.clock_scanline(),
            Self::Gxrom(m) => m.clock_scanline(),
            Self::Null => {}
        }
    }

    pub fn has_chr_ram(&self) -> bool {
        match self {
            Self::Nrom(m) => m.has_chr_ram(),
            Self::UxRom(m) => m.has_chr_ram(),
            Self::Cnrom(m) => m.has_chr_ram(),
            Self::Mmc1(m) => m.has_chr_ram(),
            Self::Mmc3(m) => m.has_chr_ram(),
            Self::Axrom(m) => m.has_chr_ram(),
            Self::Gxrom(m) => m.has_chr_ram(),
            Self::Null => true,
        }
    }
}
