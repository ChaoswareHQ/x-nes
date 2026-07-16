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
    pub last_bus_value: u8,
    pub scanline: u16,
    pub cycle: u16,
    pub nmi_pending: bool,
    pub frame_complete: bool,
    pub frame: [u8; 61440],
    odd_frame: bool,

    // Sprite rendering state (from previous scanline's evaluation)
    sprite_count: u8,
    sprite_indices: [u8; 8],
    sprite_zero_hit_possible: bool,

    // Sprite evaluation state (for NEXT scanline)
    next_sprite_count: u8,
    next_sprite_indices: [u8; 8],
    secondary_oam: [u8; 32],
    sec_oam_addr: u8,
    sprite_eval_active: bool,
    sprite_eval_index: u8,
    sprite_oam_copy: u8,
    sprite_in_range: bool,
    sprite0_found: bool,
    eval_done: bool,

    // Background shift registers
    bg_shift_low: u16,
    bg_shift_high: u16,
    bg_attr_latch: u8,
    bg_attr_shift_low: u16,
    bg_attr_shift_high: u16,

    // Tile prefetch data
    prefetch_nt: u8,
    prefetch_attr: u8,
    prefetch_pattern_low: u8,
    prefetch_pattern_high: u8,

    // VBL suppression
    vbl_suppressed: bool,

    // Previous rendering state for edge detection
    prev_rendering_enabled: bool,
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
            next_sprite_count: 0,
            next_sprite_indices: [0; 8],
            secondary_oam: [0xFF; 32],
            sec_oam_addr: 0,
            sprite_eval_active: false,
            sprite_eval_index: 0,
            sprite_oam_copy: 0,
            sprite_in_range: false,
            sprite0_found: false,
            eval_done: false,
            bg_shift_low: 0,
            bg_shift_high: 0,
            bg_attr_latch: 0,
            bg_attr_shift_low: 0,
            bg_attr_shift_high: 0,
            prefetch_nt: 0,
            prefetch_attr: 0,
            prefetch_pattern_low: 0,
            prefetch_pattern_high: 0,
            vbl_suppressed: false,
            prev_rendering_enabled: false,
        }
    }

    fn rendering_enabled(&self) -> bool {
        self.mask & 0x18 != 0
    }

    fn rendering_or_prerender(&self) -> bool {
        self.scanline < VISIBLE_SCANLINES || self.scanline == PRERENDER_SCANLINE
    }

    // ---- On-the-fly background pixel computation (reliable, uses t register) ----
    fn compute_bg_pixel(&self, x: u16, y: u16, mapper: &mut Mapper) -> (u8, u8) {
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

    // ---- Sprite rendering (reads from OAM via sprite_indices) ----
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

    fn render_pixel(&mut self, x: u16, y: u16, mapper: &mut Mapper) {
        let bg_enabled = self.mask & 0x08 != 0;
        let show_left = self.mask & 0x02 != 0;

        let (bg_pixel, bg_colour) = if !bg_enabled || (!show_left && x < 8) {
            (0, self.palette[0])
        } else {
            // Use on-the-fly computation (shift register pipeline WIP)
            self.compute_bg_pixel(x, y, mapper)
        };

        let colour = if self.mask & 0x10 != 0 && (self.mask & 0x04 != 0 || x >= 8) {
            let (sprite_colour, sp0_pixel) =
                self.render_sprite_pixel(x, bg_pixel, bg_colour, mapper);
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

    // ---- Scroll register helpers ----
    fn increment_coarse_x(&mut self) {
        if (self.v & 0x001F) == 31 {
            self.v &= !0x001F;
            self.v ^= 0x0400;
        } else {
            self.v += 1;
        }
    }

    fn increment_coarse_y(&mut self) {
        if (self.v & 0x03E0) == 0x03C0 {
            self.v &= !0x03E0;
            self.v ^= 0x0800;
        } else if (self.v & 0x03E0) == 0x03A0 {
            self.v = (self.v & !0x03E0) | 0x03C0;
        } else {
            self.v += 0x0020;
        }
        let fine_y = (self.v >> 12) & 7;
        if fine_y == 7 {
            self.v &= !0x7000;
        } else {
            self.v += 0x1000;
        }
    }

    fn copy_horizontal(&mut self) {
        self.v = (self.v & !0x041F) | (self.t & 0x041F);
    }

    fn copy_vertical(&mut self) {
        self.v = (self.v & !0x7BE0) | (self.t & 0x7BE0);
    }

    // ---- Fetch tile data from nametable + pattern tables ----
    #[allow(clippy::too_many_arguments)]
    fn fetch_tile_data(
        &self,
        tile_x: u16,
        tile_y: u16,
        fine_y: u16,
        nt: u16,
        mapper: &mut Mapper,
    ) -> (u8, u8, u8) {
        let mirroring = mapper.mirroring();
        let nt_base = 0x2000 | (nt << 10);
        let nt_addr = nt_base | (tile_y << 5) | tile_x;
        let nt_byte = self.ppu_read_nt(nt_addr, mirroring);

        let attr_addr = nt_base | 0x03C0 | ((tile_y >> 2) << 3) | (tile_x >> 2);
        let attr = self.ppu_read_nt(attr_addr, mirroring);
        let attr_shift = ((tile_x & 2) >> 1) | ((tile_y & 2) << 1);
        let attr_bits = (attr >> attr_shift) & 3;

        let bg_table = if self.ctrl & 0x10 != 0 {
            0x1000
        } else {
            0x0000
        };
        let tile_addr = bg_table | ((nt_byte as u16) << 4) | fine_y;
        let pattern_low = mapper.ppu_read(tile_addr);
        let pattern_high = mapper.ppu_read(tile_addr | 0x0008);

        (attr_bits, pattern_low, pattern_high)
    }

    fn load_tile_into_shifters(&mut self, pattern_low: u8, pattern_high: u8, attr: u8) {
        // Upper 8 bits receive new tile data; lower 8 bits are current tile being shifted out
        self.bg_shift_low = (self.bg_shift_low & 0x00FF) | ((pattern_low as u16) << 8);
        self.bg_shift_high = (self.bg_shift_high & 0x00FF) | ((pattern_high as u16) << 8);
        self.bg_attr_shift_low = (self.bg_attr_shift_low & 0x00FF) | (((attr & 1) as u16) << 8);
        self.bg_attr_shift_high =
            (self.bg_attr_shift_high & 0x00FF) | ((((attr >> 1) & 1) as u16) << 8);
    }

    // ---- Fetch one background tile at current v position ----
    fn fetch_bg_tile(&mut self, mapper: &mut Mapper) {
        let tile_x = self.v & 0x001F;
        let tile_y = (self.v >> 5) & 0x001F;
        let fine_y = (self.v >> 12) & 0x0007;
        let nt = (self.v >> 10) & 0x0003;
        let (attr_bits, pattern_low, pattern_high) =
            self.fetch_tile_data(tile_x, tile_y, fine_y, nt, mapper);
        self.load_tile_into_shifters(pattern_low, pattern_high, attr_bits);
    }

    // ---- Sprite evaluation helpers (for incremental eval during cycles 1-256) ----
    fn start_sprite_eval(&mut self) {
        self.sec_oam_addr = 0;
        self.sprite_eval_index = 0;
        self.sprite_eval_active = false;
        self.sprite_in_range = false;
        self.sprite0_found = false;
        self.eval_done = false;
        self.sprite_oam_copy = 0;
    }

    fn transfer_sprite_data(&mut self) {
        self.sprite_count = self.next_sprite_count;
        self.sprite_indices = self.next_sprite_indices;
    }

    fn evaluate_one_sprite_step(&mut self) {
        if self.eval_done || self.sprite_eval_index >= 64 {
            return;
        }

        let i = self.sprite_eval_index as usize;
        let oam_byte = self.oam[i * 4 + self.sprite_eval_active as usize];

        if self.sprite_eval_active {
            // Writing to secondary OAM or handling overflow
            if self.sec_oam_addr < 32 {
                self.secondary_oam[self.sec_oam_addr as usize] = self.sprite_oam_copy;
                self.sec_oam_addr += 1;

                if self.sprite_in_range && self.sec_oam_addr.trailing_zeros() >= 2 {
                    let idx = (self.sec_oam_addr >> 2) as usize;
                    if idx <= 8 && self.sprite_eval_index == 0 {
                        self.sprite0_found = true;
                    }
                    if idx <= 8 {
                        self.next_sprite_indices[idx.wrapping_sub(1)] = self.sprite_eval_index * 4;
                    }
                    self.sprite_in_range = false;
                }
            } else {
                if self.sprite_in_range {
                    self.status |= 0x20;
                }
                self.eval_done = true;
                self.sprite_eval_index = 64;
                return;
            }
            self.sprite_eval_active = false;
            self.sprite_eval_index += 1;
        } else {
            // Reading Y position (first byte of sprite entry)
            let sl = self.scanline;
            let sprite_h = if self.ctrl & 0x20 != 0 { 16 } else { 8 };
            self.sprite_in_range = sl >= oam_byte as u16 && sl < (oam_byte as u16 + sprite_h);
            self.sprite_oam_copy = oam_byte;
            self.sprite_eval_active = true;
        }

        if self.sprite_eval_index >= 64 {
            self.eval_done = true;
            self.next_sprite_count = (self.sec_oam_addr >> 2).min(8);
        }
    }

    // ---- Main tick function (cycle-accurate) ----
    pub fn tick(&mut self, mapper: &mut Mapper) {
        let sl = self.scanline;
        let cy = self.cycle;

        // === Cycle advance / scanline management ===
        if cy > 339 {
            self.cycle = 0;
            let ns = sl.wrapping_add(1);
            if ns > 261 {
                self.scanline = 0;
                self.odd_frame = !self.odd_frame;
                self.frame_complete = true;
                // Reset sprite eval state for new frame
                self.sprite_count = 0;
            } else {
                self.scanline = ns;
            }

            // === Cycle 0 operations ===
            if self.scanline == PRERENDER_SCANLINE {
                self.status &= !0xE0; // Clear VBL, sprite 0 hit, sprite overflow
                self.sprite_zero_hit_possible = true;
                if self.rendering_enabled() {
                    self.copy_vertical();
                }
            }
            return;
        }

        self.cycle += 1;

        // === Cycles 1-256: Visible rendering + sprite evaluation ===
        if self.rendering_or_prerender() && cy >= 1 && cy <= 256 {
            // Load tile data every cycle (approximation of fetch pipeline)
            // On the 8th cycle of each tile: load next tile and increment scroll
            if self.rendering_enabled() && (cy & 7) == 1 {
                self.fetch_bg_tile(mapper);
                self.increment_coarse_x();
            }

            // Render pixel on visible scanlines
            if sl < VISIBLE_SCANLINES {
                self.render_pixel(cy - 1, sl, mapper);
            }

            // Sprite evaluation (cycles 65-256 after secondary OAM clear)
            if cy >= 1 && cy <= 64 {
                // Clear secondary OAM (cycles 1-64)
                if self.mask & 0x10 != 0 {
                    self.secondary_oam[(cy - 1) as usize] = 0xFF;
                }
            } else if cy == 65 {
                // Start sprite evaluation for the NEXT scanline
                self.next_sprite_count = 0;
                self.next_sprite_indices = [0; 8];
                self.start_sprite_eval();
            } else if cy > 65 && cy <= 256 {
                // Incremental sprite evaluation
                if self.mask & 0x10 != 0 {
                    self.evaluate_one_sprite_step();
                }
            }

            // At cycle 256: increment vertical scroll
            if cy == 256 && self.rendering_enabled() {
                self.increment_coarse_y();
            }
        }

        // === Cycle 257: Transfer sprite eval results, copy horizontal scroll ===
        if cy == 257 {
            if self.rendering_enabled() {
                self.copy_horizontal();
            }
            // Transfer sprite data from evaluation to rendering
            // (evaluated during this scanline for use on the next scanline)
            self.transfer_sprite_data();
        }

        // === Cycle 280-304 on prerender scanline: Copy vertical scroll ===
        if sl == PRERENDER_SCANLINE && cy >= 280 && cy <= 304 && self.rendering_enabled() {
            self.copy_vertical();
        }

        // === VBlank set at cycle 1 of scanline 241 ===
        if cy == 1 && sl == VBLANK_START {
            if !self.vbl_suppressed {
                self.status = (self.status | 0x80) & !0x20;
                if self.ctrl & 0x80 != 0 {
                    self.nmi_pending = true;
                }
            }
            self.vbl_suppressed = false;
        }
    }

    pub fn tick_batch(&mut self, mut count: u16, mapper: &mut Mapper) {
        while count > 0 {
            self.tick(mapper);
            count -= 1;
        }
    }

    // ---- Memory/RAM access helpers ----
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
        let result = (s & 0xE0) | (self.last_bus_value & 0x1F);
        self.last_bus_value = result;
        // VBL suppression: if $2002 read at VBlank set cycle, suppress the flag
        if self.scanline == VBLANK_START && self.cycle == 1 && (s & 0x80) == 0 {
            self.vbl_suppressed = true;
        }
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
        if addr < 0x3F00 {
            self.data_buffer = val;
        } else {
            self.data_buffer = self.ppu_read_nt(addr & 0x2FFF, mapper.mirroring());
        }
        self.last_bus_value = result;
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
        let val = self.oam[self.oam_addr as usize];
        self.last_bus_value = val;
        val
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
