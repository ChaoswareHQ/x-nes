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
    /// PPU cycle counter for open bus decay
    pub tick_count: u64,
    /// Tick when last_bus_value was last written
    pub last_bus_write_tick: u64,
    pub scanline: u16,
    pub cycle: u16,
    /// NMI edge-detect latch: set when the PPU's NMI output transitions
    /// from high (1) to low (0). The CPU samples this latch on the
    /// penultimate cycle of each instruction.
    pub nmi_latched: bool,
    /// Previous NMI output state for edge detection.
    nmi_output: bool,
    /// Set when NMI is latched by VBlank starting (fires immediately).
    pub nmi_from_vblank: bool,
    /// Set when penultimate-cycle sample finds nmi_latched (from $2000 write).
    /// This deferred NMI fires at end of the NEXT instruction.
    pub nmi_deferred_pending: bool,
    pub frame_complete: bool,
    pub frame: [u8; 61440],
    odd_frame: bool,

    // Sprite rendering state
    sprite_count: u8,
    sprite_indices: [u8; 8],
    sprite_zero_hit_possible: bool,

    // Background shift registers
    bg_shift_low: u16,
    bg_shift_high: u16,
    bg_attr_shift_low: u16,
    bg_attr_shift_high: u16,

    // VBL suppression
    vbl_suppressed: bool,

    // Snapshot of v register at start of scanline (for stable scroll)
    render_v: u16,
    // Snapshot of fine_x at sync points (cycle 0 of prerender, cycle 257)
    render_fine_x: u8,
    // Whether rendering was enabled when the current frame started (for odd frame skip)
    frame_rendering_enabled: bool,
}

impl Ppu {
    /// Recompute NMI output (VBlank_active AND NMI_enabled) and
    /// detect rising edge (0→1 transition in NMI enable signal),
    /// which corresponds to falling edge on /NMI pin.
    /// Sets nmi_latched when edge is detected.
    /// Sets nmi_from_vblank when the edge comes from VBlank starting.
    #[inline(always)]
    pub fn update_nmi_edge(&mut self, from_vblank: bool) {
        let new_output = (self.status & 0x80 != 0) && (self.ctrl & 0x80 != 0);
        let was_active = self.nmi_output;
        // Rising edge on enable signal = falling edge on /NMI
        let edge = !was_active && new_output;
        if edge {
            self.nmi_latched = true;
            if from_vblank {
                self.nmi_from_vblank = true;
            }
        }
        self.nmi_output = new_output;
    }

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
            tick_count: 0,
            last_bus_write_tick: 0,
            scanline: 0,
            cycle: 0,
            nmi_latched: false,
            nmi_output: false,
            nmi_from_vblank: false,
            nmi_deferred_pending: false,
            frame_complete: false,
            frame: [0; 61440],
            odd_frame: true,
            sprite_count: 0,
            sprite_indices: [0; 8],
            sprite_zero_hit_possible: true,
            bg_shift_low: 0,
            bg_shift_high: 0,
            bg_attr_shift_low: 0,
            bg_attr_shift_high: 0,
            vbl_suppressed: false,
            render_v: 0,
            render_fine_x: 0,
            frame_rendering_enabled: false,
        }
    }

    fn rendering_enabled(&self) -> bool {
        self.mask & 0x18 != 0
    }

    fn rendering_or_prerender(&self) -> bool {
        self.scanline < VISIBLE_SCANLINES || self.scanline == PRERENDER_SCANLINE
    }

    // ---- On-the-fly background pixel computation ----
    fn compute_bg_pixel(&self, x: u16, y: u16, mapper: &mut Mapper) -> (u8, u8) {
        // Use render_v (snapshot of v at last sync point) instead of t.
        // During visible rendering, $2005 writes modify t but the real PPU
        // defers scroll changes to sync points:
        //   - Cycle 257: copy_horizontal (t → v coarse_x/fine_x)
        //   - Cycle 0 of prerender: copy_vertical (t → v coarse_y/fine_y)
        // Using render_v ensures scroll updates take effect at the correct
        // scanline boundary, fixing Castlevania II scroll corruption.
        let coarse_x = self.render_v & 0x001F;
        let coarse_y = (self.render_v >> 5) & 0x001F;
        let fine_y = (self.render_v >> 12) & 0x0007;
        let nt = (self.render_v >> 10) & 0x0003;

        let world_x = (coarse_x << 3) + self.render_fine_x as u16 + x;
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

    fn evaluate_sprites_for(&mut self, sl: u16) {
        self.sprite_count = 0;
        if self.mask & 0x10 == 0 {
            return;
        }
        let sprite_h = if self.ctrl & 0x20 != 0 { 16 } else { 8 };
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

    // ---- Scroll register operations ----
    fn increment_coarse_x(&mut self) {
        if (self.v & 0x001F) == 31 {
            self.v = (self.v & !0x001F) ^ 0x0400;
        } else {
            self.v += 1;
        }
    }
    fn increment_coarse_y(&mut self) {
        // Real NES increment_coarse_y: first handle fine_y, then coarse_y.
        // But the order doesn't matter for the same-tick result.
        let y = self.v & 0x03E0;
        self.v = if y == 0x03C0 {
            // coarse_y = 30: wrap to 0, toggle vertical NT (bit 11)
            (self.v & !0x03E0) ^ 0x0800
        } else if y == 0x03E0 {
            // coarse_y = 31: wrap to 0, toggle horizontal NT (bit 10)
            (self.v & !0x03E0) ^ 0x0400
        } else {
            self.v + 0x0020
        };
        // Handle fine_y (bits 12-14)
        if (self.v >> 12) & 7 == 7 {
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

    fn fetch_bg_tile(&mut self, mapper: &mut Mapper) {
        let tile_x = self.v & 0x001F;
        let tile_y = (self.v >> 5) & 0x001F;
        let fine_y = (self.v >> 12) & 0x0007;
        let nt = (self.v >> 10) & 0x0003;
        let (_, attr_bits, pat_low, pat_high) =
            self.fetch_tile_pattern(tile_x, tile_y, fine_y, nt, mapper);
        self.bg_shift_low = (self.bg_shift_low & 0x00FF) | ((pat_low as u16) << 8);
        self.bg_shift_high = (self.bg_shift_high & 0x00FF) | ((pat_high as u16) << 8);
        self.bg_attr_shift_low =
            (self.bg_attr_shift_low & 0x00FF) | (((attr_bits & 1) as u16) << 8);
        self.bg_attr_shift_high =
            (self.bg_attr_shift_high & 0x00FF) | ((((attr_bits >> 1) & 1) as u16) << 8);
    }

    #[allow(clippy::too_many_arguments)]
    fn fetch_tile_pattern(
        &self,
        tile_x: u16,
        tile_y: u16,
        fine_y: u16,
        nt: u16,
        mapper: &mut Mapper,
    ) -> (u8, u8, u8, u8) {
        let mirroring = mapper.mirroring();
        let nt_base = 0x2000 | (nt << 10);
        let nt_byte = self.ppu_read_nt(nt_base | (tile_y << 5) | tile_x, mirroring);
        let attr = self.ppu_read_nt(
            nt_base | 0x03C0 | ((tile_y >> 2) << 3) | (tile_x >> 2),
            mirroring,
        );
        let attr_shift = ((tile_x & 2) >> 1) | ((tile_y & 2) << 1);
        let attr_bits = (attr >> attr_shift) & 3;
        let bg_table = if self.ctrl & 0x10 != 0 {
            0x1000
        } else {
            0x0000
        };
        let tile_addr = bg_table | ((nt_byte as u16) << 4) | fine_y;
        let pat_low = mapper.ppu_read(tile_addr);
        let pat_high = mapper.ppu_read(tile_addr | 0x0008);
        (nt_byte, attr_bits, pat_low, pat_high)
    }

    // ---- Main cycle-accurate tick ----
    /// Get the PPU open bus value with decay applied.
    /// Bits decay toward 0 over ~1 second (~60 frames) due to capacitance.
    pub fn get_open_bus(&self) -> u8 {
        const DECAY_CYCLES: u64 = 5_000_000; // ~60 frames worth of PPU ticks
        let elapsed = self.tick_count.saturating_sub(self.last_bus_write_tick);
        if elapsed >= DECAY_CYCLES {
            return 0;
        }
        // Gradual decay: bits fade proportionally
        let decay = (elapsed * 255 / DECAY_CYCLES) as u8;
        self.last_bus_value & !(decay | decay >> 1 | decay >> 2)
    }

    pub fn tick(&mut self, mapper: &mut Mapper) {
        self.tick_count += 1;
        let sl = self.scanline;
        let cy = self.cycle;

        // ===== Cycle advance / scanline management =====
        // On odd frames with rendering enabled, the prerender scanline (261)
        // has 340 cycles instead of 341. Cycle 340 is skipped.
        // The skip is checked AFTER processing cycle 339 (when cy == 339
        // and we're about to increment to cy == 340 for the next tick).
        // We achieve this by checking cy == 339 at the START of the tick:
        // on skip, process cycle 339, then advance (skipping the cy=340 tick).
        // On normal, just process and let the next tick (cy=340) handle advance.
        if cy > 339 {
            self.cycle = 0;
            let ns = sl.wrapping_add(1);
            if ns > 261 {
                self.scanline = 0;
                self.odd_frame = !self.odd_frame;
                self.frame_complete = true;
            } else {
                self.scanline = ns;
            }
            // Cycle 0 of scanline 261 (prerender): vertical scroll copy, clear flags
            if self.scanline == PRERENDER_SCANLINE {
                self.sprite_zero_hit_possible = true;
                if self.rendering_enabled() {
                    self.copy_vertical();
                }
                // Snapshot initial scroll after copy_vertical at the prerender.
                self.render_v = self.v;
                // Also snapshot fine_x (from VBlank $2005 writes).
                // This ensures fine_x and coarse_x are in sync.
                self.render_fine_x = self.fine_x;
            }
            return;
        }
        self.cycle += 1;

        // Odd frame skip: after processing cycle 339 (now cy=340),
        // and we're on prerender + odd frame + rendering, skip forward.
        if self.scanline == PRERENDER_SCANLINE
            && self.cycle == 340
            && self.odd_frame
            && (self.mask & 0x18) != 0
        {
            // Skip cycle 340 entirely - go directly to scanline 0
            // render_v is NOT updated here - it retains the initial scroll
            // from cycle 0 of prerender + horizontal bits from cycle 257.
            self.cycle = 0;
            self.scanline = 0;
            self.odd_frame = !self.odd_frame;
            self.frame_complete = true;
            self.sprite_zero_hit_possible = true;
            return;
        }

        // ===== Cycle 1: VBlank set / prerender VBL clear =====
        if cy == 1 && sl == VBLANK_START {
            if !self.vbl_suppressed {
                self.status = (self.status | 0x80) & !0x20;
                // Update NMI edge detection (VBlank just started - immediate NMI)
                self.update_nmi_edge(true);
            }
            self.vbl_suppressed = false;
        }
        if cy == 1 && sl == PRERENDER_SCANLINE {
            // Clear VBL, sprite 0 hit, sprite overflow at cycle 1 of prerender
            self.status &= !0xE0;
            // Update NMI edge detection (VBlank clearing - not from VBlank start)
            self.update_nmi_edge(false);
        }

        // ===== Cycles 1-256: Visible rendering on visible/prerender scanlines =====
        if self.rendering_or_prerender() {
            if cy >= 1 && cy <= 256 {
                // Fetch next tile every 8 cycles (shift register pipeline)
                if self.rendering_enabled() && (cy & 7) == 1 {
                    self.fetch_bg_tile(mapper);
                    self.increment_coarse_x();
                }
                // Shift registers
                self.bg_shift_low <<= 1;
                self.bg_shift_high <<= 1;
                self.bg_attr_shift_low <<= 1;
                self.bg_attr_shift_high <<= 1;
                // Render pixel on visible scanlines
                if sl < VISIBLE_SCANLINES {
                    self.render_pixel(cy - 1, sl, mapper);
                }
                // At cycle 256: increment vertical scroll
                // NOTE: render_v is NOT updated here - it keeps the initial scroll
                // from cycle 0 of prerender. The scanline progression (y) in
                // compute_bg_pixel handles fine_y/coarse_y advancement.
                if cy == 256 && self.rendering_enabled() {
                    self.increment_coarse_y();
                }
            }

            // ===== Cycle 257: Copy horizontal scroll, evaluate sprites for NEXT scanline =====
            if cy == 257 {
                if self.rendering_enabled() {
                    self.copy_horizontal();
                    // Copy ONLY horizontal bits from v to render_v (coarse_x + NT select)
                    // This reflects $2005 writes that update t, now copied to v.
                    // Vertical bits (fine_y, coarse_y) must NOT be copied here
                    // because compute_bg_pixel uses y + fine_y to determine scanline.
                    self.render_v = (self.render_v & !0x041F) | (self.v & 0x041F);
                    // Sync fine_x with the new horizontal scroll
                    self.render_fine_x = self.fine_x;
                }
                // Evaluate sprites for the next scanline
                let next_sl = if sl == PRERENDER_SCANLINE { 0 } else { sl + 1 };
                self.evaluate_sprites_for(next_sl);
            }
        }
    }

    pub fn tick_batch(&mut self, mut count: u16, mapper: &mut Mapper) {
        while count > 0 {
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
            self.palette[if i & 0x13 == 0x10 { i & 0x0F } else { i }]
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

    /// Set the PPU data bus value and track the write tick for open bus decay.
    #[inline(always)]
    fn set_last_bus_value(&mut self, val: u8) {
        self.last_bus_value = val;
        self.last_bus_write_tick = self.tick_count;
    }

    pub fn read_status(&mut self) -> u8 {
        let s = self.status;
        self.status &= !0x80;
        self.w = 0;
        let result = (s & 0xE0) | (self.get_open_bus() & 0x1F);
        // $2002 read does NOT reset the open bus decay timer
        self.last_bus_value = result;
        // VBlank suppression: if $2002 is read on the same PPU cycle that
        // VBlank would be set (scanline 241, cycle 1), the VBlank is suppressed.
        if self.scanline == VBLANK_START && self.cycle == 1 && (s & 0x80) == 0 {
            self.vbl_suppressed = true;
        }
        // Update NMI edge detection (clearing VBlank - not from VBlank start)
        self.update_nmi_edge(false);
        result
    }

    pub fn read_data(&mut self, mapper: &mut Mapper) -> u8 {
        let addr = self.v & 0x3FFF;
        let val = if addr < 0x2000 {
            mapper.ppu_read(addr)
        } else {
            self.ppu_read_nt(addr, mapper.mirroring())
        };
        let result = if addr < 0x3F00 {
            self.data_buffer
        } else {
            // Palette read: high bits from open bus (decayed), low bits from palette
            (self.get_open_bus() & 0xC0) | (val & 0x3F)
        };
        if addr < 0x3F00 {
            self.data_buffer = val;
        } else {
            self.data_buffer = self.ppu_read_nt(addr & 0x2FFF, mapper.mirroring());
        }
        // $2007 reads DO refresh the bus (unlike $2002)
        self.set_last_bus_value(result);
        self.v = self
            .v
            .wrapping_add(if self.ctrl & 0x04 != 0 { 32 } else { 1 });
        result
    }

    pub fn write_ctrl(&mut self, val: u8) {
        self.set_last_bus_value(val);
        self.ctrl = val;
        self.t = (self.t & 0xF3FF) | ((val as u16 & 3) << 10);
        // Update NMI edge detection. $2000 writes are from_vblank=false
        // so nmi_from_vblank is NOT set. The NMI from $2000 writes is
        // deferred to the next instruction (penultimate-cycle rule).
        self.update_nmi_edge(false);
        // If NMI is being disabled, clear the latch (NMI line goes high)
        if val & 0x80 == 0 {
            self.nmi_latched = false;
        }
    }

    pub fn write_mask(&mut self, val: u8) {
        self.set_last_bus_value(val);
        self.mask = val;
    }
    pub fn write_oam_addr(&mut self, val: u8) {
        self.set_last_bus_value(val);
        self.oam_addr = val;
    }

    pub fn write_oam_data(&mut self, val: u8) {
        self.set_last_bus_value(val);
        self.oam[self.oam_addr as usize] = val;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    pub fn read_oam_data(&mut self) -> u8 {
        let mut val = self.oam[self.oam_addr as usize];
        // Bits 2-4 of sprite attribute bytes are always 0 when read via $2004
        if self.oam_addr & 0x03 == 2 {
            val &= 0xE3;
        }
        self.set_last_bus_value(val);
        val
    }

    pub fn write_scroll(&mut self, val: u8) {
        self.set_last_bus_value(val);
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
        self.set_last_bus_value(val);
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
        self.set_last_bus_value(val);
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
