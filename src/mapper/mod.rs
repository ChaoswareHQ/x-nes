use alloc::boxed::Box;

pub mod axrom;
pub mod cnrom;
pub mod gxrom;
pub mod mmc1;
pub mod mmc3;
pub mod mmc5;
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

    // MMC5-specific extensions with default no-op implementations:

    /// Called once per scanline by the PPU (for MMC5 scanline IRQ).
    fn notify_scanline(&mut self, _scanline: u16) {}

    /// Returns the nametable mapping register value.
    /// 0xFF = standard mirroring (use `mirroring()`).
    /// Otherwise, each 2-bit pair maps NT0-3 to source (0=CIRAM_A, 1=CIRAM_B, 2=ExRAM, 3=Fill).
    fn nt_mapping(&self) -> u8 {
        0xFF
    }

    /// Read from a non-CIRAM nametable source (ExRAM or fill mode).
    fn read_nt_ext(&mut self, _addr: u16, _nt_source: u8) -> u8 {
        0
    }

    /// Write to a non-CIRAM nametable source (ExRAM).
    fn write_nt_ext(&mut self, _addr: u16, _nt_source: u8, _val: u8) {}

    /// Set CHR fetch mode to background (for ExRAM mode 1).
    fn set_chr_fetch_bg(&mut self) {}

    /// Set CHR fetch mode to sprite (for ExRAM mode 1).
    fn set_chr_fetch_sprite(&mut self) {}

    /// Set the extended CHR bank from ExRAM byte (for ExRAM mode 1).
    fn set_extended_chr_bank(&mut self, _bank: u8) {}

    /// Get the extended CHR bank.
    fn get_extended_chr_bank(&self) -> u8 {
        0
    }

    /// Get the ExRAM mode.
    fn get_ex_ram_mode(&self) -> u8 {
        0
    }

    /// Get fill mode tile value.
    fn get_fill_tile(&self) -> u8 {
        0
    }

    /// Get fill mode attribute value.
    fn get_fill_attr(&self) -> u8 {
        0
    }

    /// Read a byte from ExRAM at the given offset.
    fn read_ex_ram_byte(&mut self, _offset: u16) -> u8 {
        0
    }
}

pub enum Mapper {
    Nrom(Box<nrom::Nrom>),
    UxRom(Box<uxrom::UxRom>),
    Cnrom(Box<cnrom::Cnrom>),
    Mmc1(Box<mmc1::Mmc1>),
    Mmc3(Box<mmc3::Mmc3>),
    Mmc5(Box<mmc5::Mmc5>),
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
            5 => Self::Mmc5(Box::new(mmc5::Mmc5::new(
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
            Self::Mmc5(m) => m.cpu_read(addr),
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
            Self::Mmc5(m) => m.cpu_write(addr, val),
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
            Self::Mmc5(m) => m.ppu_read(addr),
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
            Self::Mmc5(m) => m.ppu_write(addr, val),
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
            Self::Mmc5(m) => m.mirroring(),
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
            Self::Mmc5(m) => m.irq_pending(),
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
            Self::Mmc5(m) => m.ack_irq(),
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
            Self::Mmc5(m) => m.clock_scanline(),
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
            Self::Mmc5(m) => m.has_chr_ram(),
            Self::Axrom(m) => m.has_chr_ram(),
            Self::Gxrom(m) => m.has_chr_ram(),
            Self::Null => true,
        }
    }

    pub fn notify_scanline(&mut self, scanline: u16) {
        match self {
            Self::Mmc5(m) => m.notify_scanline(scanline),
            _ => {}
        }
    }

    pub fn nt_mapping(&self) -> u8 {
        match self {
            Self::Mmc5(m) => m.nt_mapping(),
            _ => 0xFF,
        }
    }

    pub fn read_nt_ext(&mut self, addr: u16, nt_source: u8) -> u8 {
        match self {
            Self::Mmc5(m) => m.read_nt_ext(addr, nt_source),
            _ => 0,
        }
    }

    pub fn write_nt_ext(&mut self, addr: u16, nt_source: u8, val: u8) {
        match self {
            Self::Mmc5(m) => m.write_nt_ext(addr, nt_source, val),
            _ => {}
        }
    }

    pub fn set_chr_fetch_bg(&mut self) {
        match self {
            Self::Mmc5(m) => m.set_chr_fetch_bg(),
            _ => {}
        }
    }

    pub fn set_chr_fetch_sprite(&mut self) {
        match self {
            Self::Mmc5(m) => m.set_chr_fetch_sprite(),
            _ => {}
        }
    }

    pub fn set_extended_chr_bank(&mut self, bank: u8) {
        match self {
            Self::Mmc5(m) => m.set_extended_chr_bank(bank),
            _ => {}
        }
    }

    pub fn get_extended_chr_bank(&self) -> u8 {
        match self {
            Self::Mmc5(m) => m.get_extended_chr_bank(),
            _ => 0,
        }
    }

    pub fn get_ex_ram_mode(&self) -> u8 {
        match self {
            Self::Mmc5(m) => m.get_ex_ram_mode(),
            _ => 0,
        }
    }

    pub fn get_fill_tile(&self) -> u8 {
        match self {
            Self::Mmc5(m) => m.get_fill_tile(),
            _ => 0,
        }
    }

    pub fn get_fill_attr(&self) -> u8 {
        match self {
            Self::Mmc5(m) => m.get_fill_attr(),
            _ => 0,
        }
    }

    pub fn read_ex_ram_byte(&mut self, offset: u16) -> u8 {
        match self {
            Self::Mmc5(m) => m.read_ex_ram_byte(offset),
            _ => 0,
        }
    }
}
