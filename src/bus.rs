use crate::apu::Apu;
use crate::controller::Gamepad;
use crate::mapper::Mapper;
use crate::ppu::Ppu;

#[allow(dead_code)]
pub struct Bus {
    pub ram: [u8; 2048],
    pub mapper: Mapper,
    pub ppu: Ppu,
    pub apu: Apu,
    pub pad1: Gamepad,
    pub pad2: Gamepad,
    /// Tracks last value on the data bus for open bus reads
    open_bus_val: u8,
    /// Set to true when DMC DMA fires between instructions
    /// Used by SHA/SHS/SHY/SHX to know if H should be ignored
    pub dmc_just_fired: bool,
    /// Global CPU cycle counter (incremented each bus access)
    pub cpu_cycle: u64,
    /// Last CPU cycle the PPU was synced to
    ppu_sync_cycle: u64,
    /// CPU cycle at which to sample NMI latch (penultimate cycle of instruction)
    pub penultimate_sample_cycle: u64,
    /// Per-access DMC ticks applied during current SH instruction.
    /// Incremented by SH instructions after each bus access.
    pub dmc_ticks: u16,
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
            open_bus_val: 0,
            dmc_just_fired: false,
            cpu_cycle: 0,
            ppu_sync_cycle: 0,
            penultimate_sample_cycle: 0,
            dmc_ticks: 0,
        }
    }

    /// Advance PPU to catch up to the current `cpu_cycle`.
    /// Each CPU cycle = 3 PPU dots. This only advances the PPU
    /// for cycles that haven't been synced yet.
    #[inline(always)]
    pub fn catch_up_ppu(&mut self) {
        if self.cpu_cycle > self.ppu_sync_cycle {
            let ppu_dots = (self.cpu_cycle - self.ppu_sync_cycle) * 3;
            self.ppu.tick_batch(ppu_dots as u16, &mut self.mapper);
            self.ppu_sync_cycle = self.cpu_cycle;
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
                0x4015 => self.apu.read(addr) | (self.open_bus_val & 0x20),
                0x4016 => (self.pad1.read() & 0x01) | (self.open_bus_val & 0xFE),
                // $4017 reads Famicom controller 2 (also NES expansion port)
                // Bit 0 = controller 2 data, bits 1-7 = open bus
                0x4017 => (self.pad2.read() & 0x01) | (self.open_bus_val & 0xFE),
                // $4000-$4014 ($4015 handled above) are write-only APU registers
                // Reading write-only APU registers returns open bus
                _ => self.open_bus_val,
            },
            4 if addr < 0x4020 => {
                // $4018-$401F: open bus, mirrors of APU registers
                self.open_bus_val
            }
            _ => {
                if addr >= 0x6000 {
                    self.mapper.cpu_read(addr)
                } else if addr >= 0x4000 {
                    // $4020-$5FFF: open bus range
                    self.open_bus_val
                } else {
                    0
                }
            }
        }
    }

    /// Sample the NMI latch at the penultimate cycle.
    #[inline(always)]
    fn sample_penultimate(&mut self) {
        if self.cpu_cycle > 0
            && (self.cpu_cycle - 1) == self.penultimate_sample_cycle
            && self.ppu.nmi_latched
        {
            self.ppu.nmi_latched = false;
            self.ppu.nmi_deferred_pending = true;
        }
    }

    /// Advance one CPU cycle, catching up the PPU by 3 dots.
    /// Call for internal cycles (no bus access).
    #[inline(always)]
    pub fn advance_cycle(&mut self) {
        self.cpu_cycle += 1;
        self.sample_penultimate();
        self.catch_up_ppu();
    }

    #[inline(always)]
    pub fn read(&mut self, addr: u16) -> u8 {
        self.cpu_cycle += 1;
        self.sample_penultimate();
        self.catch_up_ppu();
        let val = self.read_mapped(addr);
        // Update open bus tracking
        let top = (addr >> 12) as u8;
        let is_open_bus = match addr {
            0x4015 => true,
            0x4016 => false,
            _ if top == 4 && addr < 0x4018 => true,
            _ if top == 4 && addr >= 0x4018 && addr < 0x4020 => true,
            _ if top == 4 && addr >= 0x4020 && addr < 0x6000 => true,
            _ if addr < 0x4000 => false,
            _ if addr >= 0x6000 => false,
            _ => true,
        };
        if !is_open_bus {
            self.open_bus_val = val;
        }
        val
    }

    #[inline(always)]
    pub fn write(&mut self, addr: u16, val: u8) {
        self.cpu_cycle += 1;
        self.sample_penultimate();
        self.catch_up_ppu();
        let top = (addr >> 12) as u8;
        match top {
            0 | 1 => self.ram[(addr & 0x07FF) as usize] = val,
            2 | 3 => self.write_ppu(addr, val),
            4 if addr < 0x4020 => match addr {
                0x4014 => self.oam_dma(val),
                0x4016 => {
                    self.pad1.write(val);
                    self.pad2.write(val);
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
        if (addr & 0xFF00) != 0x0100 {
            self.open_bus_val = val;
        }
    }

    #[inline(always)]
    fn read_ppu(&mut self, addr: u16) -> u8 {
        self.catch_up_ppu();
        match addr & 7 {
            2 => self.ppu.read_status(),
            4 => self.ppu.read_oam_data(),
            7 => self.ppu.read_data(&mut self.mapper),
            _ => self.ppu.get_open_bus(),
        }
    }

    #[inline(always)]
    fn write_ppu(&mut self, addr: u16, val: u8) {
        self.catch_up_ppu();
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
        self.ppu.last_bus_value = val;
        self.ppu.last_bus_write_tick = self.ppu.tick_count;
    }

    fn oam_dma(&mut self, page: u8) {
        let base = (page as u16) << 8;
        for i in 0..256 {
            let addr = base | i;
            let val = self.read(addr);
            self.cpu_cycle += 1;
            self.ppu.oam[self.ppu.oam_addr as usize] = val;
            self.ppu.oam_addr = self.ppu.oam_addr.wrapping_add(1);
            self.ppu_tick(6);
            self.apu.tick(2);
        }
    }

    pub fn poll_irq(&mut self) -> bool {
        if self.apu.apu_irq_pending() {
            return true;
        }
        if self.mapper.irq_pending() {
            self.mapper.ack_irq();
            return true;
        }
        false
    }

    /// Check and perform DMC DMA if needed.
    /// Returns the number of extra CPU cycles consumed by DMA
    pub fn dmc_tick(&mut self) -> u8 {
        if self.apu.dmc_dma_pending() {
            let addr = self.apu.dmc_sample_address();
            self.cpu_cycle += 4;
            self.ppu_tick(12);
            let val = self.mapper.cpu_read(addr);
            self.apu.dmc_complete_dma(val);
            self.open_bus_val = val;
            self.dmc_just_fired = true;
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
