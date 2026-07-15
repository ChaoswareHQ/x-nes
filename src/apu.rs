const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

// DMC rate table: period in CPU cycles between DMA reads
// Indexed by bits 3-0 of $4010
const DMC_RATES: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 144, 128, 106, 96, 54, 32,
];

// LSB-first bit patterns for duty_step 0-7
// Duty 0: 0b00000010 -> 0,1,0,0,0,0,0,0  (12.5%)
// Duty 1: 0b00000110 -> 0,1,1,0,0,0,0,0  (25%)
// Duty 2: 0b00011110 -> 0,1,1,1,1,0,0,0  (50%)
// Duty 3: 0b11001111 -> 1,0,0,1,1,1,1,1  (25% negated)
const DUTY_SEQUENCES: [u8; 4] = [0x02, 0x06, 0x1E, 0xCF];

#[derive(Default, Clone)]
pub struct Pulse {
    pub enabled: bool,
    pub duty: u8,
    pub vol: u8,
    pub timer_load: u16,
    pub timer_val: u16,
    pub duty_step: u8,

    // Envelope
    pub env_start: bool,
    pub env_disable: bool, // constant volume flag (bit 4 of $4000)
    pub env_divider: u8,
    pub env_decay: u8,

    // Length counter (halt flag is bit 5 of $4000, also envelope loop flag)
    pub length_counter: u8,
    pub length_halt: bool,

    // Sweep ($4001 / $4005)
    pub sweep_enabled: bool,
    pub sweep_period: u8, // P (bits 6-4), divider period is P + 1 half-frames
    pub sweep_negate: bool,
    pub sweep_shift: u8,
    pub sweep_divider: u8,
    pub sweep_reload: bool,
}

impl Pulse {
    fn step_timer(&mut self) {
        if self.timer_val == 0 {
            self.timer_val = self.timer_load;
            self.duty_step = self.duty_step.wrapping_sub(1) & 7;
        } else {
            self.timer_val -= 1;
        }
    }

    fn clock_envelope(&mut self) {
        if self.env_start {
            self.env_start = false;
            self.env_decay = 15;
            self.env_divider = self.vol;
        } else {
            if self.env_divider == 0 {
                self.env_divider = self.vol;
                if self.env_decay == 0 {
                    if self.length_halt {
                        self.env_decay = 15;
                    }
                } else {
                    self.env_decay -= 1;
                }
            } else {
                self.env_divider -= 1;
            }
        }
    }

    fn clock_length(&mut self) {
        if self.length_counter > 0 && !self.length_halt {
            self.length_counter -= 1;
        }
    }

    fn clock_sweep(&mut self) {
        let divider_zero = self.sweep_divider == 0;

        if divider_zero && self.sweep_enabled && self.sweep_shift > 0 && !self.is_sweep_muted() {
            self.timer_load = self.sweep_calc_target();
        }

        if divider_zero || self.sweep_reload {
            self.sweep_divider = self.sweep_period;
            self.sweep_reload = false;
        } else {
            self.sweep_divider -= 1;
        }
    }

    fn is_sweep_muted(&self) -> bool {
        self.timer_load < 8 || self.sweep_calc_target() > 0x7FF
    }

    fn sweep_calc_target(&self) -> u16 {
        let shift = self.sweep_shift;
        let change = self.timer_load >> shift;
        if self.sweep_negate {
            self.timer_load - change
        } else {
            self.timer_load + change
        }
    }

    pub fn volume_output(&self) -> u8 {
        if !self.enabled || self.length_counter == 0 || self.timer_load < 8 || self.is_sweep_muted()
        {
            return 0;
        }
        let bit = (DUTY_SEQUENCES[self.duty as usize] >> self.duty_step) & 1;
        if bit == 0 {
            return 0;
        }
        if self.env_disable {
            self.vol
        } else {
            self.env_decay
        }
    }
}

#[derive(Default, Clone)]
pub struct Dmc {
    // Registers
    pub rate_index: u8,        // bits 3-0 of $4010
    pub loop_flag: bool,       // bit 4 of $4010
    pub irq_enable: bool,      // bit 7 of $4010
    pub dac: u8,               // 7-bit output level written to $4011
    pub sample_addr: u16,      // $4012: current sample address
    pub sample_addr_load: u16, // $4012 configured address = $C000 + val * 64
    pub sample_len: u16,       // $4013: sample length = val * 16 + 1

    // Internal state
    pub enabled: bool,
    pub sample_buffer: u8, // 1-byte buffer filled by DMA
    pub buffer_empty: bool,
    pub bits_remaining: u8, // bits left to shift out of sample_buffer (0-8)
    pub output_level: u8,   // 7-bit current output level

    // DMA state
    pub dma_needed: bool,     // DMC needs to read a byte from memory
    pub bytes_remaining: u16, // bytes left to read from sample

    // Timer
    pub timer: u16,      // counts down in CPU cycles
    pub timer_load: u16, // reload value from rate table

    // Status
    pub irq: bool,
}

impl Dmc {
    fn restart(&mut self) {
        self.sample_addr = self.sample_addr_load;
        self.bytes_remaining = self.sample_len;
        self.buffer_empty = true;
        self.bits_remaining = 0;
    }

    fn step(&mut self) -> bool {
        // Returns true if this step requires a DMA read (stealing CPU cycles)
        if self.timer > 0 {
            self.timer -= 1;
            return false;
        }

        // Reload timer (minimum 1 to avoid instant re-fire)
        self.timer = if self.timer_load == 0 {
            1
        } else {
            self.timer_load
        };

        if self.buffer_empty {
            // Need to read a byte into the buffer
            if self.bytes_remaining > 0 && !self.dma_needed {
                self.dma_needed = true;
            }
            return false;
        }

        // If buffer is not empty but no bits remain, it's empty
        if self.bits_remaining == 0 {
            self.buffer_empty = true;
            if self.bytes_remaining > 0 {
                self.dma_needed = true;
            } else if self.loop_flag {
                self.restart();
                self.dma_needed = true;
            } else if self.irq_enable {
                self.irq = true;
            }
            return false;
        }

        // Shift out one bit (LSB first)
        let bit = self.sample_buffer & 1;
        self.sample_buffer >>= 1;
        self.bits_remaining -= 1;

        // Adjust output level: +2 for 1, -2 for 0
        if bit == 1 {
            if self.output_level <= 125 {
                self.output_level += 2;
            }
        } else {
            if self.output_level >= 2 {
                self.output_level -= 2;
            }
        }

        if self.bits_remaining == 0 {
            self.buffer_empty = true;
            if self.bytes_remaining > 0 {
                self.dma_needed = true;
            } else if self.loop_flag {
                self.restart();
                self.dma_needed = true;
            } else if self.irq_enable {
                self.irq = true;
            }
        }

        false
    }

    /// Called when DMC DMA reads a byte from memory
    fn dma_read(&mut self, val: u8) {
        self.sample_buffer = val;
        self.bits_remaining = 8;
        self.buffer_empty = false;
        self.dma_needed = false;
        self.bytes_remaining = self.bytes_remaining.saturating_sub(1);
        // Advance sample address, wrapping from $FFFF to $8000
        self.sample_addr = if self.sample_addr == 0xFFFF {
            0x8000
        } else {
            self.sample_addr.wrapping_add(1)
        };
    }

    fn output(&self) -> u8 {
        if !self.enabled {
            return 0;
        }
        self.output_level
    }
}

pub struct Apu {
    pub cycles: u64,
    pub p1: Pulse,
    pub p2: Pulse,
    dmc: Dmc,
    pub audio_samples: [f32; 4096],
    pub sample_count: usize,
    sample_timer: f64,
    sample_period: f64,

    // Frame counter
    frame_cycles: u32,
    frame_mode: bool, // false = 4-step, true = 5-step
    interrupt_inhibit: bool,
    pub frame_irq: bool,
    // Write delay for $4017 (3 or 4 CPU cycles)
    fc_delay: u8,
    fc_write_pending: bool,
    fc_pending_mode: bool,
    fc_pending_inhibit: bool,
}

impl Default for Apu {
    fn default() -> Self {
        Self::new()
    }
}

impl Apu {
    pub fn new() -> Self {
        Self {
            cycles: 0,
            p1: Pulse::default(),
            p2: Pulse::default(),
            dmc: Dmc {
                buffer_empty: true,
                timer: 1,
                timer_load: DMC_RATES[0],
                ..Dmc::default()
            },
            audio_samples: [0.0; 4096],
            sample_count: 0,
            sample_timer: 0.0,
            sample_period: 40.584,
            frame_cycles: 0,
            frame_mode: false,
            interrupt_inhibit: false,
            frame_irq: false,
            fc_delay: 0,
            fc_write_pending: false,
            fc_pending_mode: false,
            fc_pending_inhibit: false,
        }
    }

    pub fn set_sample_rate(&mut self, rate: f64) {
        self.sample_period = 1_789_773.0 / rate;
    }

    pub fn dmc_dma_pending(&self) -> bool {
        self.dmc.dma_needed
    }

    /// Complete a DMC DMA read. `val` is the byte read from the sample address.
    pub fn dmc_complete_dma(&mut self, val: u8) {
        self.dmc.dma_read(val);
    }

    pub fn dmc_sample_address(&self) -> u16 {
        self.dmc.sample_addr
    }

    fn clock_quarter_frame(&mut self) {
        self.p1.clock_envelope();
        self.p2.clock_envelope();
    }

    fn clock_half_frame(&mut self) {
        self.p1.clock_length();
        self.p2.clock_length();
        self.p1.clock_sweep();
        self.p2.clock_sweep();
    }

    fn clock_frame_counter(&mut self) {
        self.frame_cycles += 1;

        if self.frame_mode {
            // Mode 1: 5-step sequence (18641 APU cycles)
            match self.frame_cycles {
                3729 | 7457 | 11186 | 18641 => self.clock_quarter_frame(),
                _ => {}
            }
            match self.frame_cycles {
                7457 | 18641 => self.clock_half_frame(),
                _ => {}
            }
            if self.frame_cycles >= 18641 {
                self.frame_cycles = 0;
            }
        } else {
            // Mode 0: 4-step sequence (14915 APU cycles)
            // Events happen at: step end APU cycles 3729, 7457, 11186, 14914
            match self.frame_cycles {
                3729 | 7457 | 11186 | 14914 => self.clock_quarter_frame(),
                _ => {}
            }
            match self.frame_cycles {
                7457 | 14914 => self.clock_half_frame(),
                _ => {}
            }
            // IRQ fires at APU cycle 14914 of the frame
            if self.frame_cycles == 14914 && !self.interrupt_inhibit {
                self.frame_irq = true;
            }
            if self.frame_cycles >= 14914 {
                self.frame_cycles = 0;
            }
        }
    }

    pub fn tick(&mut self, cpu_cycles: u16) {
        for _ in 0..cpu_cycles {
            self.cycles += 1;

            // Handle $4017 write delay (count down in CPU cycles)
            if self.fc_delay > 0 {
                self.fc_delay -= 1;
                if self.fc_delay == 0 && self.fc_write_pending {
                    // Apply the delayed $4017 write
                    self.frame_mode = self.fc_pending_mode;
                    self.interrupt_inhibit = self.fc_pending_inhibit;
                    if self.interrupt_inhibit {
                        self.frame_irq = false;
                    }
                    self.frame_cycles = 0;
                    if self.frame_mode {
                        self.clock_quarter_frame();
                        self.clock_half_frame();
                    }
                    self.fc_write_pending = false;
                }
            }

            // DMC step runs every CPU cycle
            self.dmc.step();

            // APU runs at half CPU rate (2 CPU cycles = 1 APU cycle)
            if self.cycles & 1 == 1 {
                self.p1.step_timer();
                self.p2.step_timer();
                self.clock_frame_counter();
            }

            self.sample_timer += 1.0;
            if self.sample_timer >= self.sample_period {
                self.sample_timer -= self.sample_period;
                if self.sample_count < self.audio_samples.len() {
                    let out = self.mixer_output();
                    self.audio_samples[self.sample_count] = out;
                    self.sample_count += 1;
                }
            }
        }
    }

    fn mixer_output(&self) -> f32 {
        let pulse1 = self.p1.volume_output() as f32;
        let pulse2 = self.p2.volume_output() as f32;
        let dmc_out = self.dmc.output() as f32;
        let pulse_sum = pulse1 + pulse2;
        // Non-linear NES DAC formula from NESDev wiki
        let pulse_part = if pulse_sum == 0.0 {
            0.0
        } else {
            95.88 / (8128.0 / pulse_sum + 100.0)
        };
        let dmc_part = if dmc_out == 0.0 {
            0.0
        } else {
            159.79 / (1.0 / (dmc_out / 8227.0) + 100.0)
        };
        pulse_part + dmc_part
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x4015 => {
                let mut status = 0u8;
                if self.p1.length_counter > 0 {
                    status |= 1;
                }
                if self.p2.length_counter > 0 {
                    status |= 2;
                }
                if self.dmc.enabled && self.dmc.bytes_remaining > 0 {
                    status |= 0x10;
                }
                if self.dmc.irq {
                    status |= 0x80;
                }
                if self.frame_irq {
                    status |= 0x40;
                }
                self.frame_irq = false;
                self.dmc.irq = false;
                status
            }
            // $4016 and $4017 reads go through the controller, but APU catches
            // reads when they come through the bus. Return open bus.
            _ => 0,
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x4000 => {
                self.p1.duty = val >> 6;
                self.p1.length_halt = val & 0x20 != 0;
                self.p1.env_disable = val & 0x10 != 0;
                self.p1.vol = val & 0x0F;
            }
            0x4001 => {
                self.p1.sweep_enabled = val & 0x80 != 0;
                self.p1.sweep_period = (val >> 4) & 7;
                self.p1.sweep_negate = val & 0x08 != 0;
                self.p1.sweep_shift = val & 0x07;
                self.p1.sweep_reload = true;
            }
            0x4002 => {
                self.p1.timer_load = (self.p1.timer_load & 0xFF00) | val as u16;
            }
            0x4003 => {
                self.p1.timer_load = (self.p1.timer_load & 0x00FF) | ((val as u16 & 7) << 8);
                self.p1.duty_step = 0;
                self.p1.env_start = true;
                if self.p1.enabled {
                    let idx = (val >> 3) as usize;
                    self.p1.length_counter = LENGTH_TABLE[idx.min(31)];
                }
            }
            0x4004 => {
                self.p2.duty = val >> 6;
                self.p2.length_halt = val & 0x20 != 0;
                self.p2.env_disable = val & 0x10 != 0;
                self.p2.vol = val & 0x0F;
            }
            0x4005 => {
                self.p2.sweep_enabled = val & 0x80 != 0;
                self.p2.sweep_period = (val >> 4) & 7;
                self.p2.sweep_negate = val & 0x08 != 0;
                self.p2.sweep_shift = val & 0x07;
                self.p2.sweep_reload = true;
            }
            0x4006 => {
                self.p2.timer_load = (self.p2.timer_load & 0xFF00) | val as u16;
            }
            0x4007 => {
                self.p2.timer_load = (self.p2.timer_load & 0x00FF) | ((val as u16 & 7) << 8);
                self.p2.duty_step = 0;
                self.p2.env_start = true;
                if self.p2.enabled {
                    let idx = (val >> 3) as usize;
                    self.p2.length_counter = LENGTH_TABLE[idx.min(31)];
                }
            }
            0x4010 => {
                // DMC control
                self.dmc.irq_enable = val & 0x80 != 0;
                self.dmc.loop_flag = val & 0x40 != 0;
                self.dmc.rate_index = val & 0x0F;
                self.dmc.timer_load = DMC_RATES[self.dmc.rate_index as usize];
                if !self.dmc.irq_enable {
                    self.dmc.irq = false;
                }
            }
            0x4011 => {
                // DMC DAC (7-bit output level)
                self.dmc.dac = val & 0x7F;
                self.dmc.output_level = self.dmc.dac;
            }
            0x4012 => {
                // DMC sample address: $C000 + val * 64
                self.dmc.sample_addr_load = 0xC000 | ((val as u16) << 6);
            }
            0x4013 => {
                // DMC sample length: val * 16 + 1
                self.dmc.sample_len = (val as u16) * 16 + 1;
            }
            0x4015 => {
                self.p1.enabled = val & 1 != 0;
                self.p2.enabled = val & 2 != 0;
                if !self.p1.enabled {
                    self.p1.length_counter = 0;
                }
                if !self.p2.enabled {
                    self.p2.length_counter = 0;
                }
                // DMC enable (bit 4)
                let prev_enabled = self.dmc.enabled;
                self.dmc.enabled = val & 0x10 != 0;
                if self.dmc.enabled && !prev_enabled {
                    // Restart DMC sample
                    self.dmc.restart();
                } else if !self.dmc.enabled {
                    // Stop DMC
                    self.dmc.bytes_remaining = 0;
                    self.dmc.bits_remaining = 0;
                    self.dmc.buffer_empty = true;
                    self.dmc.dma_needed = false;
                }
                // Clear DMC IRQ
                self.dmc.irq = false;
            }
            0x4017 => {
                // Frame counter write with delay
                // Delay is 3 CPU cycles for write (put) cycles, 4 for read (get) cycles
                // Since $4017 is always written during a put cycle, use 3
                // Store the pending values but don't apply yet
                self.fc_delay = 3;
                self.fc_write_pending = true;
                self.fc_pending_mode = val & 0x80 != 0;
                self.fc_pending_inhibit = val & 0x40 != 0;
            }
            _ => {}
        }
    }
}
