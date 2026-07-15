use crate::mapper::Mapper;

const VISIBLE_SCANLINES: u16 = 240;
const VBLANK_START: u16 = 241;
const PRERENDER_SCANLINE: u16 = 261;

#[allow(dead_code)]
pub struct Ppu {
    pub vram: [u8; 0x1000],
    pub palette: [u8; 0x20],
    pub oam: [u8; 0x100],

    pub ctrl: u8,
    pub mask: u8,
    pub status: u8,
    pub oam_addr: u8,

    pub v: u16,
    pub t: u16,
    pub fine_x: u8,
    pub w: u8,

    pub data_buffer: u8,
    /// Last value written to any PPU register (for open bus reads)
    pub last_bus_value: u8,

    pub scanline: u16,
    pub cycle: u16,
    pub nmi_pending: bool,
    pub frame_complete: bool,
    pub frame: [u8; 61440],
    odd_frame: bool,

    sprite_count: u8,
    sprite_indices: [u8; 8],
    sprite_zero_hit_possible: bool,
    vset: bool,
    vset_latch1: bool,
    vset_latch2: bool,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            vram: [0; 0x1000],
            palette: [0; 0x20],
            oam: [0; 0x100],
            ctrl: 0,
            mask: 0,
            status: 0,
            oam_addr: 0,
            v: 0,
            t: 0,
            fine_x: 0,
            w: 0,
            data_buffer: 0,
            last_bus_value: 0,
            scanline: 0,
            cycle: 0,
            nmi_pending: false,
            frame_complete: false,
            frame: [0; 61440],
            odd_frame: true,
            sprite_count: 0,
            sprite_indices: [0; 8],
            sprite_zero_hit_possible: true,
            vset: false,
            vset_latch1: false,
            vset_latch2: false,
        }
    }

    fn rendering_enabled(&self) -> bool {
        self.mask & 0x18 != 0
    }

    fn rendering_or_prerender(&self) -> bool {
        self.scanline < VISIBLE_SCANLINES || self.scanline == PRERENDER_SCANLINE
    }

    fn get_bg_pixel(&self, x: u16, y: u16, mapper: &mut Mapper) -> (u8, u8) {
        let coarse_x = self.t & 0x001F;
        let coarse_y = (self.t >> 5) & 0x001F;
        let fine_y = (self.t >> 12) & 0x0007;
        let nt = (self.t >> 10) & 0x0003;

        let world_x = (coarse_x << 3) + self.fine_x as u16 + x;
        let world_y = (coarse_y << 3) + fine_y + y;

        let mirroring = mapper.mirroring();
        let mut actual_nt = nt;
        if (world_x >> 8) & 1 != 0 {
            actual_nt ^= 1;
        }
        let coarse_y_calc = world_y >> 3;
        if (coarse_y_calc / 30) & 1 != 0 {
            actual_nt ^= 2;
        }

        let tile_x = (world_x >> 3) & 31;
        let tile_y = (coarse_y_calc % 30) & 31;
        let pixel_x = world_x & 7;
        let pixel_y = world_y & 7;

        let nt_base = 0x2000 | (actual_nt << 10);
        let nt_addr = nt_base | (tile_y << 5) | tile_x;
        let tile_index = self.ppu_read_nt(nt_addr, mirroring);

        let bg_table = if self.ctrl & 0x10 != 0 {
            0x1000
        } else {
            0x0000
        };
        let tile_addr = bg_table | ((tile_index as u16) << 4) | pixel_y;
        let low = mapper.ppu_read(tile_addr);
        let high = mapper.ppu_read(tile_addr | 0x0008);

        let shift = 7 - pixel_x;
        let pixel = ((high >> shift) & 1) << 1 | ((low >> shift) & 1);

        if pixel == 0 {
            (0, self.palette[0])
        } else {
            let attr_addr = nt_base | 0x03C0 | ((tile_y >> 2) << 3) | (tile_x >> 2);
            let attr = self.ppu_read_nt(attr_addr, mirroring);
            let s = ((tile_x >> 1) & 1) << 1 | ((tile_y >> 1) & 1) << 2;
            let pal_group = (attr >> s) & 3;
            (
                pixel,
                self.palette[((pal_group as usize) << 2) | pixel as usize],
            )
        }
    }

    /// Returns `(colour, sp0_pixel)` — `sp0_pixel` is non-zero when sprite 0 has a visible pixel here
    fn render_sprite_pixel(
        &self,
        x: u16,
        bg_pixel: u8,
        bg_color: u8,
        mapper: &mut Mapper,
    ) -> (u8, u8) {
        let use_16 = self.ctrl & 0x20 != 0;
        let sprite_h = if use_16 { 16 } else { 8 };
        let sl = self.scanline;
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
            let sy_off = sl.wrapping_sub(sy) as u8;
            let pixel_y = if flip_y {
                (sprite_h as u8).wrapping_sub(1).wrapping_sub(sy_off)
            } else {
                sy_off
            };
            let pixel_x = if flip_x {
                7u8.wrapping_sub(x.wrapping_sub(sx) as u8)
            } else {
                x.wrapping_sub(sx) as u8
            };
            let tile_addr = if use_16 {
                let bank = if tile & 1 != 0 { 0x1000 } else { 0x0000 };
                let base_tile = tile & 0xFE;
                let bottom = u16::from(pixel_y >= 8);
                let fine_y = pixel_y & 7;
                bank | ((base_tile + bottom) << 4) | fine_y as u16
            } else {
                let bank = if self.ctrl & 0x08 != 0 {
                    0x1000
                } else {
                    0x0000
                };
                bank | (tile << 4) | pixel_y as u16
            };
            let low = mapper.ppu_read(tile_addr);
            let high = mapper.ppu_read(tile_addr | 8);
            let shift = 7 - pixel_x;
            let pixel = ((high >> shift) & 1) << 1 | ((low >> shift) & 1);
            if pixel == 0 {
                continue;
            }
            let sp0_here = if idx == 0 { pixel } else { 0 };
            if behind && bg_pixel != 0 {
                return (bg_color, sp0_here);
            }
            return (
                self.palette[0x10 | ((palette_bits as usize) << 2) | pixel as usize],
                sp0_here,
            );
        }
        (bg_color, 0)
    }

    pub fn tick(&mut self, mapper: &mut Mapper) {
        let sl = self.scanline;
        let cy = self.cycle;

        if self.odd_frame && self.rendering_enabled() && sl == 0 && cy == 0 {
            self.cycle = 1;
            return;
        }

        self.vset_latch1 = !self.vset;
        if self.vset && !self.vset_latch2 {
            self.status |= 0x80;
        }
        self.vset_latch2 = !self.vset_latch1;

        if sl == VBLANK_START && cy == 1 {
            self.status |= 0x80;
            self.vset = true;
            if self.ctrl & 0x80 != 0 {
                self.nmi_pending = true;
            }
        }

        if sl == PRERENDER_SCANLINE && cy == 1 {
            self.status &= !0xC0;
            self.vset = false;
            self.sprite_zero_hit_possible = true;
        }

        if self.rendering_or_prerender() {
            if sl < VISIBLE_SCANLINES {
                if cy == 0 {
                    self.evaluate_sprites(sl);
                }
                if cy > 0 && cy <= 256 {
                    self.render_pixel(cy - 1, sl, mapper);
                }
            } else if sl == PRERENDER_SCANLINE && cy == 0 {
                self.evaluate_sprites(sl);
            }
        }

        let nc = cy.wrapping_add(1);
        if nc > 340 {
            self.cycle = 0;
            let ns = sl.wrapping_add(1);
            if ns > 261 {
                self.scanline = 0;
                self.odd_frame = !self.odd_frame;
                self.frame_complete = true;
            } else {
                self.scanline = ns;
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
        let sprite_h = if self.ctrl & 0x20 != 0 { 16 } else { 8 };
        for i in (0..0x100).step_by(4) {
            let sy = self.oam[i] as u16;
            if sy <= sl && sl < sy + sprite_h {
                if self.sprite_count < 8 {
                    let idx = self.sprite_count as usize;
                    self.sprite_indices[idx] = (i >> 2) as u8;
                    self.sprite_count += 1;
                } else {
                    self.status |= 0x20;
                }
            }
        }
    }

    fn render_pixel(&mut self, x: u16, y: u16, mapper: &mut Mapper) {
        let bg_enabled = self.mask & 0x08 != 0;
        let show_left = self.mask & 0x02 != 0;
        let (bg_pixel, bg_colour) = if !bg_enabled || (!show_left && x < 8) {
            (0, self.palette[0])
        } else {
            self.get_bg_pixel(x, y, mapper)
        };
        let colour = if self.mask & 0x10 != 0 && (self.mask & 0x04 != 0 || x >= 8) {
            let (sprite_colour, sp0_pixel) =
                self.render_sprite_pixel(x, bg_pixel, bg_colour, mapper);
            // Sprite 0 hit: bg and sprite both non-zero, once per frame, not at pixel 255
            if sp0_pixel != 0 && bg_pixel != 0 && x != 255 && self.sprite_zero_hit_possible {
                self.status |= 0x40;
                self.sprite_zero_hit_possible = false;
            }
            sprite_colour
        } else {
            bg_colour
        };
        self.frame[(y as usize) * 256 + (x as usize)] = colour;
    }

    pub fn tick_batch(&mut self, mut count: u16, mapper: &mut Mapper) {
        while count > 0 {
            let cy = self.cycle;
            if cy >= 257 && cy <= 340 && self.rendering_or_prerender() {
                // During HBlank (cycles 257-340), sprite evaluation data is loaded
                // but no pixels are rendered. Still need to advance cycles for
                // proper VBlank timing.
            }
            self.tick(mapper);
            count -= 1;
        }
    }

    fn nt_index(addr: u16, mirroring: u8) -> (usize, u16) {
        let a = addr & 0x3FFF;
        let (nt, off) = match mirroring {
            0 => ((a >> 11) & 1, a & 0x03FF),
            _ => ((a >> 10) & 1, a & 0x03FF),
        };
        (nt as usize, off)
    }

    pub fn ppu_read_nt(&self, addr: u16, mirroring: u8) -> u8 {
        let a = addr & 0x3FFF;
        if a >= 0x3F00 {
            let i = (a & 0x1F) as usize;
            if i & 0x13 == 0x10 {
                self.palette[i & 0x0F]
            } else {
                self.palette[i]
            }
        } else if a < 0x2000 {
            0
        } else if a < 0x3000 {
            let (nt, off) = Self::nt_index(a, mirroring);
            self.vram[nt * 0x400 + off as usize]
        } else {
            self.ppu_read_nt(a & 0x2FFF, mirroring)
        }
    }

    pub fn ppu_write_nt(&mut self, addr: u16, val: u8, mirroring: u8) {
        let a = addr & 0x3FFF;
        if a >= 0x3F00 {
            let i = (a & 0x1F) as usize;
            self.palette[if i & 0x13 == 0x10 { i & 0x0F } else { i }] = val;
        } else if a < 0x2000 {
        } else if a < 0x3000 {
            let (nt, off) = Self::nt_index(a, mirroring);
            self.vram[nt * 0x400 + off as usize] = val;
        } else {
            self.ppu_write_nt(a & 0x2FFF, val, mirroring);
        }
    }

    pub fn read_status(&mut self) -> u8 {
        let s = self.status;
        self.status &= !0x80;
        self.w = 0;
        // Lower 5 bits come from PPU data bus (open bus)
        let result = (s & 0xE0) | (self.last_bus_value & 0x1F);
        self.last_bus_value = result;
        result
    }

    pub fn read_data(&mut self, mapper: &mut Mapper) -> u8 {
        let addr = self.v & 0x3FFF;
        let val = if addr < 0x2000 {
            mapper.ppu_read(addr)
        } else {
            self.ppu_read_nt(addr, mapper.mirroring())
        };
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
        self.last_bus_value = val;
        let was_nmi = self.ctrl & 0x80 != 0;
        self.ctrl = val;
        self.t = (self.t & 0xF3FF) | ((val as u16 & 3) << 10);
        if !was_nmi && val & 0x80 != 0 && self.status & 0x80 != 0 {
            self.nmi_pending = true;
        }
    }

    pub fn write_mask(&mut self, val: u8) {
        self.last_bus_value = val;
        self.mask = val;
    }
    pub fn write_oam_addr(&mut self, val: u8) {
        self.last_bus_value = val;
        self.oam_addr = val;
    }

    pub fn write_oam_data(&mut self, val: u8) {
        self.last_bus_value = val;
        self.oam[self.oam_addr as usize] = val;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    pub fn read_oam_data(&mut self) -> u8 {
        self.oam[self.oam_addr as usize]
    }

    pub fn write_scroll(&mut self, val: u8) {
        self.last_bus_value = val;
        if self.w == 0 {
            self.t = (self.t & 0xFFE0) | ((val >> 3) as u16);
            self.fine_x = val & 7;
            self.w = 1;
        } else {
            self.t =
                (self.t & 0xFC1F) | (((val as u16) & 0x07) << 12) | (((val as u16) & 0xF8) << 2);
            self.w = 0;
        }
    }

    pub fn write_addr(&mut self, val: u8) {
        self.last_bus_value = val;
        if self.w == 0 {
            self.t = ((self.t & 0x00FF) | ((val as u16) << 8)) & 0x3FFF;
            self.w = 1;
        } else {
            self.t = (self.t & 0xFF00) | val as u16;
            self.v = self.t;
            self.w = 0;
        }
    }

    pub fn write_data(&mut self, val: u8, mapper: &mut Mapper) {
        self.last_bus_value = val;
        let addr = self.v & 0x3FFF;
        if addr < 0x2000 {
            mapper.ppu_write(addr, val);
        } else {
            self.ppu_write_nt(addr, val, mapper.mirroring());
        }
        self.v = self
            .v
            .wrapping_add(if self.ctrl & 0x04 != 0 { 32 } else { 1 });
    }

    pub fn ppu_read(&mut self, addr: u16, mapper: &mut Mapper) -> u8 {
        if addr < 0x2000 {
            mapper.ppu_read(addr)
        } else {
            self.ppu_read_nt(addr, mapper.mirroring())
        }
    }

    pub fn ppu_write(&mut self, addr: u16, val: u8, mapper: &mut Mapper) {
        if addr < 0x2000 {
            mapper.ppu_write(addr, val);
        } else {
            self.ppu_write_nt(addr, val, mapper.mirroring());
        }
    }
}
