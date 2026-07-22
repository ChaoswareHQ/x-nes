use crate::mapper::Mapper;

use super::Ppu;

impl Ppu {
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
            // Use mapper.ppu_read directly (no A12 tracking) for on-the-fly
            // sprite pixel rendering. In the real NES, sprite pattern data
            // is fetched during cycles 257-320 of the previous scanline and
            // the A12 edges from those fetches are generated in
            // evaluate_sprites_for. Re-reading here for pixel output is a
            // software optimization that must not affect MMC3 IRQ timing.
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

    pub(super) fn render_pixel(&mut self, x: u16, y: u16, mapper: &mut Mapper) {
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

    pub(super) fn evaluate_sprites_for(&mut self, sl: u16, mapper: &mut Mapper) {
        self.sprite_count = 0;
        if self.mask & 0x10 == 0 {
            return;
        }
        let sprite_h = if self.ctrl & 0x20 != 0 { 16 } else { 8 };
        let use_16 = self.ctrl & 0x20 != 0;
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

        /// Generate A12 edges for sprite pattern fetches (real NES PPU does
        /// these during cycles 257-320 of the previous scanline). Each sprite
        /// slot generates 2 CHR reads (pattern low + pattern high) which
        /// clock the MMC3 scanline counter via A12 rising edges.
        /// We read through chr_read() here so that notify_mapper_a12 tracks
        /// A12 state properly for the MMC3 IRQ.
        ///
        /// The real PPU always accesses 8 sprite slots per scanline, using
        /// tile $FF for empty slots (dummy fetches). These still generate
        /// A12 edges and clock the MMC3 counter.
        //
        // Note: The forced notify_mapper_a12(0x0000) between slots is intentionally
        // omitted. The A12 glitch filter (3-cycle) in notify_mapper_a12 would
        // suppress edges from forced-low transitions since a12_low_cycles is reset
        // on every falling edge. Instead we let chr_read() naturally track A12
        // transitions, which generates exactly 1 rising edge per scanline when
        // the first sprite pattern is read (A12 goes from background $0000 to
        // sprite $1000). Subsequent sprite reads keep A12 high, matching the
        // SJNES reference emulator behavior and producing the correct MMC3 IRQ
        // timing for games like Super Mario Bros 3.
        for si in 0..8 {
            let tile_addr = if si < self.sprite_count {
                let idx = self.sprite_indices[si as usize] as usize;
                let oi = idx * 4;
                let tile = self.oam[oi + 1] as u16;
                let attr = self.oam[oi + 2];
                let flip_y = attr & 0x80 != 0;
                let pixel_y = if flip_y {
                    (sprite_h as u8).wrapping_sub(1)
                } else {
                    0
                };
                if use_16 {
                    let bank = if tile & 1 != 0 { 0x1000 } else { 0x0000 };
                    let base_tile = tile & 0xFE;
                    bank | (base_tile << 4) | pixel_y as u16
                } else {
                    let bank = if self.ctrl & 0x08 != 0 {
                        0x1000
                    } else {
                        0x0000
                    };
                    bank | (tile << 4) | pixel_y as u16
                }
            } else {
                // Dummy sprite fetch: tile $FF from sprite pattern table
                ((self.ctrl & 0x08 != 0) as u16) * 0x1000 | (0xFFu16 << 4)
            };
            // chr_read tracks A12 via notify_mapper_a12. The first sprite
            // read with A12=1 (sprite table at $1000) generates a rising edge.
            // Subsequent reads keep A12 high, so no further edges occur.
            self.chr_read(tile_addr, mapper);
            self.chr_read(tile_addr | 8, mapper);
        }
    }

    /// Fetch next background tile pattern from VRAM.
    /// This triggers mapper CHR reads (needed for MMC3 IRQ timing).
    pub(super) fn fetch_bg_tile(&mut self, mapper: &mut Mapper) {
        // Notify A12 low before nametable read (simulates address bus transition
        // from CHR $0000 region, needed for MMC3 scanline counter rising edges
        // when subsequent pattern reads are at $1000+)
        self.notify_mapper_a12(0x0000, mapper);
        // Accumulate PPU cycles between the forced A12=0 (simulating NT/AT reads)
        // and the pattern LSB read. On real NES, NT read is at cycle 1, AT at cycle 3,
        // pattern LSB at cycle 5 — a 4-cycle gap that passes the MMC3's 3-cycle A12
        // glitch filter. Without this, a12_low_cycles stays 0 and all background
        // edges would be suppressed.
        self.a12_low_cycles = self.a12_low_cycles.saturating_add(4);
        if self.a12_low_cycles > 255 {
            self.a12_low_cycles = 255;
        }

        let tile_x = self.v & 0x001F;
        let tile_y = (self.v >> 5) & 0x001F;
        let fine_y = (self.v >> 12) & 0x0007;
        let nt = (self.v >> 10) & 0x0003;
        self.fetch_tile_pattern(tile_x, tile_y, fine_y, nt, mapper);
    }

    #[allow(clippy::too_many_arguments)]
    fn fetch_tile_pattern(
        &mut self,
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
        let pat_low = self.chr_read(tile_addr, mapper);
        let pat_high = self.chr_read(tile_addr | 0x0008, mapper);
        (nt_byte, attr_bits, pat_low, pat_high)
    }
}
