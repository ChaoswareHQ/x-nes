use crate::apu::Apu;
use crate::controller::Gamepad;
use crate::mapper::Mapper;
use crate::ppu::Ppu;

pub struct Bus {
    pub ram: [u8; 2048],
    pub mapper: Mapper,
    pub ppu: Ppu,
    pub apu: Apu,
    pub pad1: Gamepad,
    pub pad2: Gamepad,
    /// Tracks last value on the data bus for open bus reads
    open_bus: u8,
    /// Set to true when DMC DMA fires between instructions
    /// Used by SHA/SHS/SHY/SHX to know if H should be ignored
    pub dmc_just_fired: bool,
}

impl Bus {
    pub fn new(mapper: Mapper) -> Self {
        Self {
            ram: [0; 2048],
            mapper,
            ppu: Ppu::new(),
            apu: Apu::new(),
            pad1: Gamepad::new(),
            pad2: Gamepad::new(),
            open_bus: 0,
            dmc_just_fired: false,
        }
    }

    #[inline(always)]
    fn read_mapped(&mut self, addr: u16) -> u8 {
        let top = (addr >> 12) as u8;
        match top {
            0 | 1 => self.ram[(addr & 0x07FF) as usize],
            2 | 3 => self.read_ppu(addr),
            4 if addr < 0x4018 => match addr {
                // $4015 returns APU status, but bit 5 is open bus (from data bus)
                0x4015 => self.apu.read(addr) | (self.open_bus & 0x20),
                0x4016 => (self.pad1.read() & 0x01) | (self.open_bus & 0xE0),
                // $4017 reads Famicom controller 2 (also NES expansion port)
                // Bit 0 = controller 2 data, bits 5-7 = open bus
                0x4017 => (self.pad2.read() & 0x01) | (self.open_bus & 0xE0),
                // $4000-$4014 ($4015 handled above) are write-only APU registers
                // Reading write-only APU registers returns open bus
                _ => self.open_bus,
            },
            4 if addr < 0x4020 => {
                // $4018-$401F: open bus, mirrors of APU registers
                self.open_bus
            }
            _ => {
                if addr >= 0x6000 {
                    self.mapper.cpu_read(addr)
                } else if addr >= 0x4000 {
                    // $4020-$5FFF: open bus range
                    self.open_bus
                } else {
                    0
                }
            }
        }
    }

    #[inline(always)]
    pub fn read(&mut self, addr: u16) -> u8 {
        let val = self.read_mapped(addr);
        // Update open bus tracking
        // Only $4015 and $4016 return actual register data that drives the bus.
        // Write-only APU registers ($4000-$4014, $4017) return open bus
        // without updating it.
        let top = (addr >> 12) as u8;
        let is_open_bus = match addr {
            // $4015 returns APU status but does NOT drive the external data bus
            // (APU has an internal data path)
            0x4015 => true,
            // $4016 returns controller data - drives the data bus
            0x4016 => false,
            // $4000-$4014, $4017 are write-only - don't drive the data bus
            _ if top == 4 && addr < 0x4018 => true,
            // $4018-$401F are mirror APU registers - open bus
            _ if top == 4 && addr >= 0x4018 && addr < 0x4020 => true,
            // $4020-$5FFF: open bus range
            _ if top == 4 && addr >= 0x4020 && addr < 0x6000 => true,
            // Regular RAM/ROM reads drive the data bus
            _ if addr < 0x4000 => false,
            // $6000+ reads from mapper drive the data bus
            _ if addr >= 0x6000 => false,
            _ => true,
        };
        if !is_open_bus {
            self.open_bus = val;
        }
        val
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
                    self.pad2.write(val); // Strobe affects both controllers
                    self.apu.write(addr, val);
                }
                _ => self.apu.write(addr, val),
            },
            _ => {
                if addr >= 0x6000 {
                    self.mapper.cpu_write(addr, val);
                }
            }
        }
        // Writes update the open bus value (CPU drives the data bus).
        // Stack writes ($100-$1FF) are excluded - the 6502's internal
        // bus transactions during JSR pushes don't propagate.
        // NOTE: top = addr >> 12, so $0100-$01FF gives top=0 (not 1!).
        // Check the actual page using (addr & 0xFF00) != 0x0100.
        if (addr & 0xFF00) != 0x0100 {
            self.open_bus = val;
        }
    }

    #[inline(always)]
    fn read_ppu(&mut self, addr: u16) -> u8 {
        match addr & 7 {
            2 => self.ppu.read_status(),
            4 => self.ppu.read_oam_data(),
            7 => self.ppu.read_data(&mut self.mapper),
            // Write-only registers return PPU open bus (last written value)
            _ => self.ppu.last_bus_value,
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
            7 => self.ppu.write_data(val, &mut self.mapper),
            _ => {}
        }
        // All writes to PPU registers update the internal data bus
        self.ppu.last_bus_value = val;
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

    pub fn poll_irq(&mut self) -> bool {
        // Check APU (frame counter) IRQ first
        if self.apu.apu_irq_pending() {
            // Don't acknowledge here - $4015 read does that
            return true;
        }
        // Then check mapper IRQ
        if self.mapper.irq_pending() {
            self.mapper.ack_irq();
            return true;
        }
        false
    }

    /// Check and perform DMC DMA if needed.
    /// Returns the number of extra CPU cycles consumed by DMA
    /// (0 for no DMA, 3-4 for a DMA cycle)
    pub fn dmc_tick(&mut self) -> u8 {
        if self.apu.dmc_dma_pending() {
            let addr = self.apu.dmc_sample_address();
            let val = self.mapper.cpu_read(addr);
            self.apu.dmc_complete_dma(val);
            // DMC DMA reads a byte from memory like a CPU read
            self.open_bus = val;
            // Flag that a DMA just fired (for SHA/SHS/SHY/SHX IgnoreH)
            self.dmc_just_fired = true;
            // DMC DMA steals 4 CPU cycles (affects PPU timing)
            self.ppu_tick(12);
            4
        } else {
            0
        }
    }

    /// Tick the PPU by `count` cycles with the mapper reference
    pub fn ppu_tick(&mut self, count: u16) {
        self.ppu.tick_batch(count, &mut self.mapper);
    }

    /// PPU read with mapper (for FFI / debuggers)
    pub fn ppu_read_mapped(&mut self, addr: u16) -> u8 {
        self.ppu.ppu_read(addr, &mut self.mapper)
    }

    /// PPU write with mapper (for FFI / debuggers)
    pub fn ppu_write_mapped(&mut self, addr: u16, val: u8) {
        self.ppu.ppu_write(addr, val, &mut self.mapper);
    }
}
