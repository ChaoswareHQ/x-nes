// PPU module - Picture Processing Unit
//
// Organized by functionality:
// - bus.rs: VRAM address space, nametable/palette access, open bus decay
// - render.rs: Background/sprite pixel rendering, tile fetching
// - registers.rs: PPU register read/write, NMI edge detection

mod bus;
mod registers;
mod render;

use crate::mapper::Mapper;

const VISIBLE_SCANLINES: u16 = 240;
const VBLANK_START: u16 = 241;
const PRERENDER_SCANLINE: u16 = 261;

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
    pub tick_count: u64,
    pub last_bus_write_tick: u64,
    pub scanline: u16,
    pub cycle: u16,
    pub nmi_latched: bool,
    nmi_output: bool,
    pub nmi_from_vblank: bool,
    pub nmi_deferred_pending: bool,
    pub frame_complete: bool,
    pub frame: [u8; 61440],
    odd_frame: bool,
    sprite_count: u8,
    sprite_indices: [u8; 8],
    sprite_zero_hit_possible: bool,
    vbl_suppressed: bool,
    render_v: u16,
    render_fine_x: u8,
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
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
            vbl_suppressed: false,
            render_v: 0,
            render_fine_x: 0,
        }
    }

    fn rendering_enabled(&self) -> bool {
        self.mask & 0x18 != 0
    }

    fn rendering_or_prerender(&self) -> bool {
        self.scanline < VISIBLE_SCANLINES || self.scanline == PRERENDER_SCANLINE
    }

    fn increment_coarse_x(&mut self) {
        if (self.v & 0x001F) == 31 {
            self.v = (self.v & !0x001F) ^ 0x0400;
        } else {
            self.v += 1;
        }
    }

    fn increment_coarse_y(&mut self) {
        let y = self.v & 0x03E0;
        self.v = if y == 0x03C0 {
            (self.v & !0x03E0) ^ 0x0800
        } else if y == 0x03E0 {
            (self.v & !0x03E0) ^ 0x0400
        } else {
            self.v + 0x0020
        };
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

    #[allow(clippy::too_many_lines)]
    pub fn tick(&mut self, mapper: &mut Mapper) {
        self.tick_count += 1;
        let sl = self.scanline;
        let cy = self.cycle;

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
            if self.scanline == PRERENDER_SCANLINE {
                self.sprite_zero_hit_possible = true;
                if self.rendering_enabled() {
                    self.copy_vertical();
                }
                self.render_v = self.v;
                self.render_fine_x = self.fine_x;
            }
            return;
        }
        self.cycle += 1;

        if self.scanline == PRERENDER_SCANLINE
            && self.cycle == 340
            && self.odd_frame
            && (self.mask & 0x18) != 0
        {
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

        if self.rendering_or_prerender() {
            if (1..=256).contains(&cy) {
                if self.rendering_enabled() && (cy & 7) == 1 {
                    self.fetch_bg_tile(mapper);
                    self.increment_coarse_x();
                }
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
}
