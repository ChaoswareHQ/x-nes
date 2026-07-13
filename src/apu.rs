pub struct Apu {
    pub cycles: u64,
}

impl Default for Apu {
    fn default() -> Self {
        Self { cycles: 0 }
    }
}

impl Apu {
    pub fn new() -> Self {
        Self { cycles: 0 }
    }

    pub fn tick(&mut self, cpu_cycles: u8) {
        self.cycles += cpu_cycles as u64;
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x4015 => {
                let s = 0;
                // TODO: read pulse/triangle/noise/DMC status
                s
            }
            _ => 0,
        }
    }

    pub fn write(&mut self, addr: u16, _val: u8) {
        match addr {
            0x4000..=0x4013 | 0x4017 => {
                // TODO: APU register writes
            }
            0x4015 => {
                // TODO: APU status write
            }
            _ => {}
        }
    }
}
