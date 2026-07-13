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

    sprite_count: u8,
    sprite_indices: [u8; 8],
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
            sprite_count: 0,
            sprite_indices: [0; 8],
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
            if cy == 0 {
                self.evaluate_sprites(sl);
            }
            if cy > 0 && cy <= 256 {
                self.render_pixel(cy - 1, sl);
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

    fn evaluate_sprites(&mut self, sl: u16) {
        self.sprite_count = 0;
        if self.mask & 0x10 == 0 {
            return;
        }
        let sprite_h = if self.ctrl & 0x20 != 0 { 16u16 } else { 8u16 };
        for i in (0..0x100).step_by(4) {
            let sy = self.oam[i] as u16;
            if sy <= sl && sl < sy + sprite_h {
                if self.sprite_count < 8 {
                    self.sprite_indices[self.sprite_count as usize] = (i >> 2) as u8;
                    self.sprite_count += 1;
                } else {
                    self.status |= 0x20;
                }
            }
        }
    }

    fn render_sprite_pixel(&self, x: u16, sl: u16, bg: u8) -> u8 {
        let use_16 = self.ctrl & 0x20 != 0;
        let sprite_h = if use_16 { 16u16 } else { 8u16 };

        for si in 0..self.sprite_count {
            let idx = self.sprite_indices[si as usize] as usize;
            let oi = idx * 4;
            let sy = self.oam[oi] as u16;
            let tile = self.oam[oi + 1] as u16;
            let attr = self.oam[oi + 2];
            let sx = self.oam[oi + 3] as u16;

            if x < sx || x >= sx + 8 {
                continue;
            }

            let palette_bits = attr & 0x03;
            let behind = attr & 0x20 != 0;
            let flip_x = attr & 0x40 != 0;
            let flip_y = attr & 0x80 != 0;

            let sy_off = sl - sy;
            let pixel_y = if flip_y {
                sprite_h - 1 - sy_off
            } else {
                sy_off
            };
            let pixel_x = if flip_x { 7 - (x - sx) } else { x - sx };

            let tile_addr = if use_16 {
                let bank = if tile & 1 != 0 { 0x1000 } else { 0x0000 };
                let t = tile & 0xFE;
                bank | (t << 4) | pixel_y
            } else {
                let bank = if self.ctrl & 0x08 != 0 {
                    0x1000
                } else {
                    0x0000
                };
                bank | (tile << 4) | pixel_y
            };

            let low = self.ppu_read_internal(tile_addr);
            let high = self.ppu_read_internal(tile_addr | 8);
            let shift = 7 - pixel_x;
            let pixel = ((high >> shift) & 1) << 1 | ((low >> shift) & 1);
            if pixel == 0 {
                continue;
            }
            if behind && bg != self.palette[0] {
                continue;
            }

            return self.palette[0x10 | ((palette_bits as usize) << 2) | pixel as usize];
        }
        bg
    }

    fn render_pixel(&mut self, x: u16, y: u16) {
        let bg_enabled = self.mask & 0x08 != 0;
        let show_left = self.mask & 0x02 != 0;

        let bg_colour = if !bg_enabled || (!show_left && x < 8) {
            0
        } else {
            self.get_bg_pixel(x, y)
        };

        if self.mask & 0x10 != 0 && (show_left || x >= 8) {
            let colour = self.render_sprite_pixel(x, y, bg_colour);
            self.frame[(y as usize) * 256 + (x as usize)] = colour;
        } else {
            self.frame[(y as usize) * 256 + (x as usize)] = bg_colour;
        }
    }

    fn get_bg_pixel(&self, x: u16, y: u16) -> u8 {
        let t = self.t;
        let fine_x_scroll = self.fine_x as u16;

        let coarse_x = t & 0x001F;
        let coarse_y = (t >> 5) & 0x001F;
        let fine_y = (t >> 12) & 0x0007;
        let nt = (t >> 10) & 0x0003;

        let world_x = (coarse_x << 3) + fine_x_scroll + x;
        let world_y = (coarse_y << 3) + fine_y + y;

        let tile_x = (world_x >> 3) & 31;
        let tile_y = (world_y >> 3) & 31;
        let pixel_x = world_x & 7;
        let pixel_y = world_y & 7;

        let nt_base = 0x2000 | (nt << 10);
        let nt_addr = nt_base | (tile_y << 5) | tile_x;
        let tile_index = self.ppu_read_internal(nt_addr);

        let bg_table = if self.ctrl & 0x10 != 0 {
            0x1000
        } else {
            0x0000
        };
        let tile_addr = bg_table | ((tile_index as u16) << 4) | pixel_y;
        let low = self.ppu_read_internal(tile_addr);
        let high = self.ppu_read_internal(tile_addr | 0x0008);

        let shift = 7 - pixel_x;
        let pixel = ((high >> shift) & 1) << 1 | ((low >> shift) & 1);

        if pixel == 0 {
            self.palette[0]
        } else {
            let attr_addr = nt_base | 0x03C0 | ((tile_y >> 2) << 3) | (tile_x >> 2);
            let attr = self.ppu_read_internal(attr_addr);
            let shift = ((tile_x >> 1) & 1) << 1 | ((tile_y >> 1) & 1) << 2;
            let pal_group = (attr >> shift) & 3;
            self.palette[((pal_group as usize) << 2) | pixel as usize]
        }
    }

    fn ppu_read_internal(&self, addr: u16) -> u8 {
        match addr & 0x3FFF {
            a @ 0x0000..=0x1FFF => self.chr_rom[a as usize],
            a @ 0x2000..=0x2FFF => {
                let nt = (self.ctrl & 3) as usize;
                self.vram[nt * 0x400 + (a & 0x03FF) as usize]
            }
            a @ 0x3000..=0x3EFF => self.ppu_read_internal(a & 0x2FFF),
            a @ 0x3F00..=0x3FFF => {
                let i = (a & 0x1F) as usize;
                self.palette[if i & 0x13 == 0x10 { i & 0x0F } else { i }]
            }
            _ => 0,
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
        let val = self.ppu_read_internal(addr);
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
        self.ppu_read_internal(addr)
    }

    pub fn ppu_write(&mut self, addr: u16, val: u8) {
        match addr & 0x3FFF {
            a @ 0x0000..=0x1FFF if self.chr_ram => {
                self.chr_rom[a as usize] = val;
            }
            _a @ 0x0000..=0x1FFF => {}
            a @ 0x2000..=0x2FFF => {
                let nt = (self.ctrl & 3) as usize;
                self.vram[nt * 0x400 + (a & 0x03FF) as usize] = val;
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
