pub struct Ppu {
    pub chr_rom: [u8; 0x2000],
    pub chr_ram: bool,
    pub vram: [u8; 0x1000],
    pub palette: [u8; 0x20],
    pub oam: [u8; 0x100],

    pub ctrl: u8,
    pub mask: u8,
    pub status: u8,
    pub oam_addr: u8,

    pub data_buffer: u8,
    pub v: u16,
    pub t: u16,
    pub fine_x: u8,
    pub w: u8,

    pub scanline: u16,
    pub cycle: u16,
    pub nmi_pending: bool,
    pub frame_complete: bool,
    pub frame: [u8; 256 * 240],
}

impl Ppu {
    pub fn new(chr_data: &[u8]) -> Self {
        let mut chr_rom = [0u8; 0x2000];
        chr_rom[..chr_data.len().min(0x2000)]
            .copy_from_slice(&chr_data[..chr_data.len().min(0x2000)]);
        Self {
            chr_rom,
            chr_ram: chr_data.is_empty(),
            vram: [0; 0x1000],
            palette: [0; 0x20],
            oam: [0; 0x100],
            ctrl: 0,
            mask: 0,
            status: 0,
            oam_addr: 0,
            data_buffer: 0,
            v: 0,
            t: 0,
            fine_x: 0,
            w: 0,
            scanline: 0,
            cycle: 0,
            nmi_pending: false,
            frame_complete: false,
            frame: [0; 256 * 240],
        }
    }

    pub fn tick(&mut self) {
        let sl = self.scanline;
        let cy = self.cycle;

        if sl < 240 {
            if sl == 0 && cy == 0 {
                self.cycle = 1;
                return;
            }
            if cy < 256 {
                self.frame[(sl as usize) * 256 + (cy as usize)] = self.status >> 7;
            }
        } else if sl == 241 && cy == 1 {
            self.status |= 0xC0;
            if self.ctrl & 0x80 != 0 {
                self.nmi_pending = true;
            }
        } else if sl == 261 && cy == 1 {
            self.status &= 0x3F;
        }

        let nc = cy.wrapping_add(1);
        if nc > 340 {
            self.cycle = 0;
            self.scanline = sl.wrapping_add(1);
            if self.scanline > 261 {
                self.scanline = 0;
                self.frame_complete = true;
            }
        } else {
            self.cycle = nc;
        }
    }

    pub fn tick_batch(&mut self, mut count: u16) {
        while count > 0 {
            let sl = self.scanline;
            let cy = self.cycle;

            if sl < 240 && (257..=340).contains(&cy) {
                let remaining = 341 - cy;
                let skip = if remaining < count { remaining } else { count };
                self.cycle = cy + skip;
                count -= skip;
                continue;
            }

            self.tick();
            count -= 1;
        }
    }

    pub fn read_status(&mut self) -> u8 {
        let s = self.status;
        self.status &= 0x7F;
        self.w = 0;
        s
    }

    pub fn read_data(&mut self) -> u8 {
        let addr = self.v & 0x3FFF;
        let val = self.ppu_read(addr);
        let result = if addr < 0x3F00 { self.data_buffer } else { val };
        if addr & 0x3F00 != 0x3F00 {
            self.data_buffer = val;
        }
        self.v = self
            .v
            .wrapping_add(if self.ctrl & 0x04 != 0 { 32 } else { 1 });
        result
    }

    pub fn write_ctrl(&mut self, val: u8) {
        let was_nmi = self.ctrl & 0x80 != 0;
        self.ctrl = val;
        self.t = (self.t & 0xF3FF) | ((val as u16 & 3) << 10);
        if !was_nmi && val & 0x80 != 0 && self.status & 0x80 != 0 {
            self.nmi_pending = true;
        }
    }

    pub fn write_mask(&mut self, val: u8) {
        self.mask = val;
    }
    pub fn write_oam_addr(&mut self, val: u8) {
        self.oam_addr = val;
    }

    pub fn write_oam_data(&mut self, val: u8) {
        self.oam[self.oam_addr as usize] = val;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    pub fn read_oam_data(&mut self) -> u8 {
        self.oam[self.oam_addr as usize]
    }

    pub fn write_scroll(&mut self, val: u8) {
        if self.w == 0 {
            self.t = (self.t & 0xFFE0) | ((val >> 3) as u16);
            self.fine_x = val & 7;
            self.w = 1;
        } else {
            self.t = (self.t & 0xFC1F) | (((val as u16) & 7) << 12) | (((val as u16) & 0xF8) << 2);
            self.w = 0;
        }
    }

    pub fn write_addr(&mut self, val: u8) {
        if self.w == 0 {
            self.t = ((self.t & 0x00FF) | ((val as u16) << 8)) & 0x3FFF;
            self.w = 1;
        } else {
            self.t = (self.t & 0xFF00) | val as u16;
            self.v = self.t;
            self.w = 0;
        }
    }

    pub fn write_data(&mut self, val: u8) {
        let addr = self.v & 0x3FFF;
        self.ppu_write(addr, val);
        self.v = self
            .v
            .wrapping_add(if self.ctrl & 0x04 != 0 { 32 } else { 1 });
    }

    pub fn ppu_read(&mut self, addr: u16) -> u8 {
        match addr & 0x3FFF {
            a @ 0x0000..=0x1FFF => self.chr_rom[a as usize],
            a @ 0x2000..=0x2FFF => {
                self.vram[((self.ctrl & 3) as usize) * 0x400 + (a & 0x03FF) as usize]
            }
            a @ 0x3000..=0x3EFF => self.ppu_read(a & 0x2FFF),
            a @ 0x3F00..=0x3FFF => {
                let i = (a & 0x1F) as usize;
                self.palette[if i & 0x13 == 0x10 { i & 0x0F } else { i }]
            }
            _ => 0,
        }
    }

    pub fn ppu_write(&mut self, addr: u16, val: u8) {
        match addr & 0x3FFF {
            a @ 0x0000..=0x1FFF if self.chr_ram => {
                self.chr_rom[a as usize] = val;
            }
            _a @ 0x0000..=0x1FFF => {}
            a @ 0x2000..=0x2FFF => {
                self.vram[((self.ctrl & 3) as usize) * 0x400 + (a & 0x03FF) as usize] = val;
            }
            a @ 0x3000..=0x3EFF => self.ppu_write(a & 0x2FFF, val),
            a @ 0x3F00..=0x3FFF => {
                let i = (a & 0x1F) as usize;
                self.palette[if i & 0x13 == 0x10 { i & 0x0F } else { i }] = val;
            }
            _ => {}
        }
    }
}
