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

// ---------------------------------------------------------------------------
// PPU timing constants
// ---------------------------------------------------------------------------
pub(crate) const VISIBLE_SCANLINES: u16 = 240;
pub(crate) const VBLANK_START: u16 = 241;
pub(crate) const PRERENDER_SCANLINE: u16 = 261;
const TOTAL_SCANLINES: u16 = 262;
const CYCLES_PER_LINE: u16 = 341;
const HORIZONTAL_COPY_CYCLE: u16 = 257;

// ---------------------------------------------------------------------------
// PPU register bit masks  (named per NESDev wiki convention)
// ---------------------------------------------------------------------------
// PPUSTATUS ($2002) bits
const STATUS_VBLANK: u8 = 0x80;
const STATUS_SPRITE0_HIT: u8 = 0x40;
const STATUS_SPRITE_OVERFLOW: u8 = 0x20;

// PPUMASK ($2001) bits
const MASK_BG_ENABLE: u8 = 0x08;
const MASK_SPRITE_ENABLE: u8 = 0x10;
const MASK_RENDERING_BITS: u8 = MASK_BG_ENABLE | MASK_SPRITE_ENABLE;

// PPUCTRL ($2000) bits
const CTRL_NMI_ENABLE: u8 = 0x80;

// PPU v register (loopy-v) bit masks
const V_COARSE_X_MASK: u16 = 0x001F;
const V_COARSE_Y_MASK: u16 = 0x03E0;
const V_HORIZONTAL_MASK: u16 = 0x041F; // coarse_x | nt_x (bits 0-4, 10)
const V_VERTICAL_MASK: u16 = 0x7BE0; // coarse_y | nt_y | fine_y (bits 5-9, 11, 12-14)
const V_COARSE_Y_WRAP: u16 = 0x03C0; // coarse_y = 30
const V_COARSE_Y_CLAMP: u16 = 0x03E0; // coarse_y = 31
const V_FINE_Y_OVERFLOW: u16 = 0x7000; // fine_y = 7

// ---------------------------------------------------------------------------
// PPU state
// ---------------------------------------------------------------------------

pub struct Ppu {
    pub vram: [u8; 0x1000],
    pub palette: [u8; 0x20],
    pub oam: [u8; 0x100],

    // Registers
    pub ctrl: u8,
    pub mask: u8,
    pub status: u8,
    pub oam_addr: u8,

    // Internal scroll / address registers
    pub v: u16, // current VRAM address (loopy-v)
    pub t: u16, // temporary VRAM address (loopy-t)
    pub fine_x: u8,
    w: u8, // write toggle (0 = first write, 1 = second)

    // Data bus / open bus
    pub data_buffer: u8,
    pub last_bus_value: u8,
    pub tick_count: u64,
    pub last_bus_write_tick: u64,

    // Scanline / cycle position
    pub scanline: u16,
    pub cycle: u16,

    // -----------------------------------------------------------------------
    // NMI state machine
    //
    //   nmi_output     = previous NMI line state (for edge detection)
    //   nmi_latched    = NMI edge detected, pending CPU sample
    //   nmi_from_vblank = True if edge was from VBlank (fires immediately)
    //   nmi_deferred   = NMI from penultimate-cycle $2000 write (fires next instr)
    // -----------------------------------------------------------------------
    pub nmi_latched: bool,
    nmi_output: bool,
    pub nmi_from_vblank: bool,
    pub nmi_deferred_pending: bool,

    // Frame tracking
    pub frame_complete: bool,
    pub frame: [u8; 61440],
    odd_frame: bool,

    // Sprite evaluation state
    sprite_count: u8,
    sprite_indices: [u8; 8],
    sprite_zero_hit_possible: bool,

    // VBlank suppression (from $2002 read on VBlank-start cycle)
    vbl_suppressed: bool,

    // Stable scroll snapshots (used by on-the-fly renderer)
    render_v: u16,
    render_fine_x: u8,

    // A12 edge tracking for mapper scanline counters (MMC3)
    prev_a12: bool,
    a12_low_cycles: u8,
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
            prev_a12: false,
            a12_low_cycles: 0,
        }
    }

    // -----------------------------------------------------------------------
    // Rendering enable helpers
    // -----------------------------------------------------------------------

    fn rendering_enabled(&self) -> bool {
        self.mask & MASK_RENDERING_BITS != 0
    }

    fn rendering_or_prerender(&self) -> bool {
        self.scanline < VISIBLE_SCANLINES || self.scanline == PRERENDER_SCANLINE
    }

    // -----------------------------------------------------------------------
    // Scroll register operations  (v register manipulation)
    // These implement the real NES PPU's v-register address arithmetic.
    // -----------------------------------------------------------------------

    fn increment_coarse_x(&mut self) {
        if (self.v & V_COARSE_X_MASK) == 31 {
            self.v = (self.v & !V_COARSE_X_MASK) ^ 0x0400;
        } else {
            self.v += 1;
        }
    }

    fn increment_coarse_y(&mut self) {
        let y = self.v & V_COARSE_Y_MASK;
        self.v = if y == V_COARSE_Y_WRAP {
            (self.v & !V_COARSE_Y_MASK) ^ 0x0800
        } else if y == V_COARSE_Y_CLAMP {
            (self.v & !V_COARSE_Y_MASK) ^ 0x0400
        } else {
            self.v + 0x0020
        };
        // Handle fine_y (bits 12-14)
        if (self.v >> 12) & 7 == 7 {
            self.v &= !V_FINE_Y_OVERFLOW;
        } else {
            self.v += 0x1000;
        }
    }

    fn copy_horizontal(&mut self) {
        self.v = (self.v & !V_HORIZONTAL_MASK) | (self.t & V_HORIZONTAL_MASK);
    }

    fn copy_vertical(&mut self) {
        self.v = (self.v & !V_VERTICAL_MASK) | (self.t & V_VERTICAL_MASK);
    }

    // -----------------------------------------------------------------------
    // NMI edge detection
    // -----------------------------------------------------------------------

    /// Update NMI output state and detect rising edge on enable signal
    /// (falling edge on `/NMI` pin). Called every time `VBlank` or NMI-enable changes.
    #[inline(always)]
    pub fn update_nmi_edge(&mut self, from_vblank: bool) {
        let new_output = (self.status & STATUS_VBLANK != 0) && (self.ctrl & CTRL_NMI_ENABLE != 0);
        let was_active = self.nmi_output;
        let edge = !was_active && new_output;
        if edge {
            self.nmi_latched = true;
            if from_vblank {
                self.nmi_from_vblank = true;
            }
        }
        self.nmi_output = new_output;
    }

    // -----------------------------------------------------------------------
    // PPU tick — cycle-accurate main loop
    //
    // Phases (per PPU dot / cycle):
    //   0..339  Normal processing
    //   340+    Scanline advance (end-of-line)
    //
    // Special cycles:
    //   1       VBlank set/clear, NMI edge
    //   1..256  Visible rendering + tile fetch
    //   256     Coarse Y increment
    //   257     Horizontal scroll copy + sprite evaluation for next line
    // -----------------------------------------------------------------------

    /// Advance to the next scanline. Called when cy exceeds 339.
    fn advance_scanline(&mut self, mapper: &mut Mapper) {
        self.cycle = 0;
        let ns = self.scanline.wrapping_add(1);
        if ns >= TOTAL_SCANLINES {
            self.scanline = 0;
            self.odd_frame = !self.odd_frame;
            self.frame_complete = true;
        } else {
            self.scanline = ns;
        }
        // Notify mapper of scanline change (MMC5 scanline IRQ)
        mapper.notify_scanline(self.scanline);
        // Cycle 0 of prerender scanline: vertical scroll copy, reset sprite hit
        if self.scanline == PRERENDER_SCANLINE {
            self.sprite_zero_hit_possible = true;
            if self.rendering_enabled() {
                self.copy_vertical();
            }
            // Snapshot scroll state for the on-the-fly renderer
            self.render_v = self.v;
            self.render_fine_x = self.fine_x;
        }
    }

    /// Handle the NES PPU odd-frame cycle skip.
    /// On odd frames with rendering enabled, the prerender scanline has 340 cycles
    /// instead of 341. Cycle 340 is skipped.
    fn handle_odd_frame_skip(&mut self, mapper: &mut Mapper) -> bool {
        if self.scanline == PRERENDER_SCANLINE
            && self.cycle == CYCLES_PER_LINE - 1 // 340
            && self.odd_frame
            && self.rendering_enabled()
        {
            self.cycle = 0;
            self.scanline = 0;
            self.odd_frame = !self.odd_frame;
            self.frame_complete = true;
            self.sprite_zero_hit_possible = true;
            mapper.notify_scanline(0);
            return true;
        }
        false
    }

    /// Cycle 1 of `VBlank` scanline: set `VBlank` flag and fire NMI.
    fn cycle1_vblank_set(&mut self) {
        if !self.vbl_suppressed {
            self.status = (self.status | STATUS_VBLANK) & !STATUS_SPRITE_OVERFLOW;
            self.update_nmi_edge(true);
        }
        self.vbl_suppressed = false;
    }

    /// Cycle 1 of prerender scanline: clear VBlank/sprite flags.
    fn cycle1_prerender_clear(&mut self) {
        self.status &= !(STATUS_VBLANK | STATUS_SPRITE0_HIT | STATUS_SPRITE_OVERFLOW);
        self.update_nmi_edge(false);
    }

    /// Cycles 1–256: visible rendering + tile fetching + pixel output.
    /// `cy` is the PPU dot being processed (pre-increment value, range 1..256).
    fn cycles_1_to_256(&mut self, cy: u16, mapper: &mut Mapper) {
        if self.rendering_enabled() && (cy & 7) == 1 {
            self.fetch_bg_tile(mapper);
            self.increment_coarse_x();
        }

        if self.scanline < VISIBLE_SCANLINES {
            let pixel_x = cy - 1;
            self.render_pixel(pixel_x, self.scanline, mapper);
        }

        if cy == 256 && self.rendering_enabled() {
            self.increment_coarse_y();
        }
    }

    /// Cycle 257: copy horizontal scroll bits, evaluate sprites for next line.
    /// Also generates A12 edges for sprite pattern fetches (MMC3 IRQ timing).
    fn cycle257_copy_and_eval(&mut self, mapper: &mut Mapper) {
        if self.rendering_enabled() {
            self.copy_horizontal();
            // Update render_v with horizontal bits only.
            // Vertical bits stay at prerender-cycle-0 snapshot because
            // compute_bg_pixel uses y + fine_y to track the scanline.
            self.render_v = (self.render_v & !V_HORIZONTAL_MASK) | (self.v & V_HORIZONTAL_MASK);
            self.render_fine_x = self.fine_x;
        }
        // Evaluate sprites for the next scanline
        let next_sl = if self.scanline == PRERENDER_SCANLINE {
            0
        } else {
            self.scanline + 1
        };
        self.evaluate_sprites_for(next_sl, mapper);
    }

    // -----------------------------------------------------------------------
    // Mapper CHR read with A12 edge detection (for MMC3 scanline IRQ, etc.)
    // -----------------------------------------------------------------------

    /// Notify the PPU that address `addr` is on the address bus (for A12 tracking).
    /// MMC3 counts A12 rising edges to clock its scanline counter.
    ///
    fn notify_mapper_a12(&mut self, addr: u16, mapper: &mut Mapper) {
        let a12_new = (addr & 0x1000) != 0;
        if a12_new && !self.prev_a12 {
            // Rising edge on A12 — only count if A12 was low for >= 3 PPU cycles
            // This filters out glitches from a single cycle-low between pattern reads
            // in the same tile fetch (attr read → pattern read gap is only 2 cycles).
            if self.a12_low_cycles >= 3 {
                mapper.clock_scanline();
            }
            self.a12_low_cycles = 0;
        } else if !a12_new && self.prev_a12 {
            self.a12_low_cycles = 0;
        }
        self.prev_a12 = a12_new;
    }

    /// Read from CHR space through the mapper with A12 edge tracking.
    pub fn chr_read(&mut self, addr: u16, mapper: &mut Mapper) -> u8 {
        self.notify_mapper_a12(addr, mapper);
        mapper.ppu_read(addr)
    }

    // ===================================================================
    // Main tick entry point
    // ===================================================================

    pub fn tick(&mut self, mapper: &mut Mapper) {
        self.tick_count += 1;

        // Notify mapper on the very first tick (scanline 0 initial state)
        if self.tick_count == 1 {
            mapper.notify_scanline(0);
        }

        // Increment A12 low-cycle counter (for MMC3 scanline IRQ)
        if !self.prev_a12 && self.a12_low_cycles < 255 {
            self.a12_low_cycles += 1;
        }

        let cy = self.cycle;
        let sl = self.scanline;

        // --- Phase 1: End-of-line scanline advance ---
        if cy > CYCLES_PER_LINE - 2 {
            self.advance_scanline(mapper);
            return;
        }
        self.cycle += 1;

        // --- Phase 2: Odd frame skip (happens at cycle 340 on prerender) ---
        if self.handle_odd_frame_skip(mapper) {
            return;
        }

        // --- Phase 3: Cycle 1 special events ---
        if cy == 1 {
            match sl {
                VBLANK_START => self.cycle1_vblank_set(),
                PRERENDER_SCANLINE => self.cycle1_prerender_clear(),
                _ => {}
            }
        }

        // --- Phase 4: Visible rendering (cycles 1-256 on visible/prerender lines) ---
        if self.rendering_or_prerender() {
            if (1..=256).contains(&cy) {
                self.cycles_1_to_256(cy, mapper);
            }
            // --- Phase 5: Cycle 257 — horizontal copy + sprite evaluation ---
            if cy == HORIZONTAL_COPY_CYCLE {
                self.cycle257_copy_and_eval(mapper);
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
