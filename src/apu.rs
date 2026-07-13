#[derive(Default)]
pub struct Pulse {
    pub enabled: bool,
    pub duty: u8,
    pub vol: u8,
    pub timer_load: u16,
    pub timer_val: u16,
    pub duty_step: u8,
    pub length_counter: u8,
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
    fn output(&self) -> f32 {
        if !self.enabled || self.timer_load < 8 || self.length_counter == 0 {
            return 0.0;
        }
        let duty_table = [0b01000000, 0b01100000, 0b01111000, 0b10011111];
        let bit = (duty_table[self.duty as usize] >> self.duty_step) & 1;
        if bit != 0 {
            (self.vol as f32) / 15.0
        } else {
            0.0
        }
    }
}

pub struct Apu {
    pub cycles: u64,
    pub p1: Pulse,
    pub p2: Pulse,
    pub audio_samples: [f32; 4096],
    pub sample_count: usize,
    sample_timer: f64,
    sample_period: f64, // CPU cycles per audio sample
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
            audio_samples: [0.0; 4096],
            sample_count: 0,
            sample_timer: 0.0,
            sample_period: 40.584, // 1789773 / 44100 default
        }
    }

    /// Set the output sample rate (e.g. 48000.0 for 48kHz).
    /// Recalculates the number of CPU cycles per audio sample.
    pub fn set_sample_rate(&mut self, rate: f64) {
        self.sample_period = 1_789_773.0 / rate;
        eprintln!("APU sample period: {} cycles/sample (rate: {}Hz)", self.sample_period, rate);
    }

    pub fn tick(&mut self, cpu_cycles: u8) {
        for _ in 0..cpu_cycles {
            self.cycles += 1;
            if self.cycles % 2 == 0 {
                self.p1.step_timer();
                self.p2.step_timer();
            }

            self.sample_timer += 1.0;
            if self.sample_timer >= self.sample_period {
                self.sample_timer -= self.sample_period;
                if self.sample_count < self.audio_samples.len() {
                    let out = (self.p1.output() + self.p2.output()) * 0.5;
                    self.audio_samples[self.sample_count] = out;
                    self.sample_count += 1;
                }
            }
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x4015 => 0,
            _ => 0,
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x4000 => {
                self.p1.duty = val >> 6;
                self.p1.vol = val & 15;
            }
            0x4002 => {
                self.p1.timer_load = (self.p1.timer_load & 0xFF00) | val as u16;
            }
            0x4003 => {
                self.p1.timer_load = (self.p1.timer_load & 0x00FF) | ((val as u16 & 7) << 8);
                self.p1.duty_step = 0;
                self.p1.length_counter = 0xFF;
            }
            0x4004 => {
                self.p2.duty = val >> 6;
                self.p2.vol = val & 15;
            }
            0x4006 => {
                self.p2.timer_load = (self.p2.timer_load & 0xFF00) | val as u16;
            }
            0x4007 => {
                self.p2.timer_load = (self.p2.timer_load & 0x00FF) | ((val as u16 & 7) << 8);
                self.p2.duty_step = 0;
                self.p2.length_counter = 0xFF;
            }
            0x4015 => {
                self.p1.enabled = val & 1 != 0;
                self.p2.enabled = val & 2 != 0;
            }
            _ => {}
        }
    }
}
