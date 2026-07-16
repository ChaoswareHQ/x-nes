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

struct NullMapper;

impl MapperImpl for NullMapper {
    fn cpu_read(&mut self, _: u16) -> u8 {
        0
    }
    fn cpu_write(&mut self, _: u16, _: u8) {}
    fn ppu_read(&mut self, _: u16) -> u8 {
        0
    }
    fn ppu_write(&mut self, _: u16, _: u8) {}
    fn mirroring(&self) -> u8 {
        0
    }
    fn irq_pending(&self) -> bool {
        false
    }
    fn ack_irq(&mut self) {}
    fn has_chr_ram(&self) -> bool {
        true
    }
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
            0 => Self::Nrom(Box::new(nrom::Nrom::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
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

    fn dispatch<T>(&mut self, f: impl FnOnce(&mut dyn MapperImpl) -> T) -> T {
        match self {
            Self::Nrom(m) => f(&mut **m),
            Self::UxRom(m) => f(&mut **m),
            Self::Cnrom(m) => f(&mut **m),
            Self::Mmc1(m) => f(&mut **m),
            Self::Mmc3(m) => f(&mut **m),
            Self::Axrom(m) => f(&mut **m),
            Self::Gxrom(m) => f(&mut **m),
            Self::Null => f(&mut NullMapper),
        }
    }

    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        self.dispatch(|m| m.cpu_read(addr))
    }

    pub fn cpu_write(&mut self, addr: u16, val: u8) {
        self.dispatch(|m| m.cpu_write(addr, val));
    }

    pub fn ppu_read(&mut self, addr: u16) -> u8 {
        self.dispatch(|m| m.ppu_read(addr))
    }

    pub fn ppu_write(&mut self, addr: u16, val: u8) {
        self.dispatch(|m| m.ppu_write(addr, val));
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
        self.dispatch(|m| m.ack_irq());
    }

    pub fn clock_scanline(&mut self) {
        self.dispatch(|m| m.clock_scanline());
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
