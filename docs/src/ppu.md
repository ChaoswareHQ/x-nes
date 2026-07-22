# 4. Picture Processing Unit

The PPU (Picture Processing Unit) generates the NES video signal. It runs at three times the CPU speed and has its own memory system for tiles, palettes, and sprites.

## 4.1 PPU Memory

The PPU has its own 16KB address space ($0000-$3FFF), separate from the CPU:

```
$0000-$0FFF   4KB     Pattern Table 0 (256 tiles, 16 bytes each)
$1000-$1FFF   4KB     Pattern Table 1 (256 tiles)
$2000-$23BF   960B    Name Table 0 (tile indices)
$23C0-$23FF   64B     Attribute Table 0 (palette assignments per 2×2 tile group)
$2400-$27BF   960B    Name Table 1
$27C0-$27FF   64B     Attribute Table 1
$2800-$2BFF            Name Table 2 (usually mirror of 0)
$2C00-$2FFF            Name Table 3 (usually mirror of 1)
$3F00-$3F0F   16B     Background palette
$3F10-$3F1F   16B     Sprite palette (mirrors $3F00/$3F04/$3F08/$3F0C)
```

### 4.1.1 Pattern Tables

Tiles are 8×8 pixels, each pixel being 2 bits (selecting one of 4 colors from a palette). A tile takes 16 bytes: 8 bytes for the low bitplane (bits 0-7), 8 bytes for the high bitplane (bits 8-15).

```
Tile at $0000:
  Byte 0:  $7E → bitplane 0 row 0: 01111110
  Byte 1:  $FF → bitplane 0 row 1: 11111111
  ...
  Byte 7:  $42 → bitplane 0 row 7: 01000010
  Byte 8:  $00 → bitplane 1 row 0: 00000000
  Byte 9:  $00 → bitplane 1 row 1: 00000000
  ...
  Byte 15: $00 → bitplane 1 row 7: 00000000

Pixel (x,y) color = ((bitplane1[y] >> (7-x)) & 1) << 1
                  |  ((bitplane0[y] >> (7-x)) & 1)
                 = value 0-3 (transparent, color 1, color 2, color 3)
```

### 4.1.2 Name Tables

A name table is a 32×30 grid of tile indices (960 bytes). Each byte selects which 8×8 tile to draw from the pattern tables. The PPU's current name table is selected by bits 1-0 of PPUCTRL ($2000).

### 4.1.3 Attribute Tables

Attribute tables assign palettes to 2×2 tile groups (4 tiles). Each byte covers a 32×32 pixel region (4×4 tile groups). Each 2-bit field selects one of 4 background palettes:

```
Attribute byte for 4×4 tile groups:
  Bits 0-1: Top-left 2×2 group
  Bits 2-3: Top-right 2×2 group
  Bits 4-5: Bottom-left 2×2 group
  Bits 6-7: Bottom-right 2×2 group
```

### 4.1.4 Palette RAM

The NES has 32 palette entries. Addresses $3F10/$3F14/$3F18/$3F1C mirror $3F00/$3F04/$3F08/$3F0C:

```
$3F00:  Universal background color (used for color 0 everywhere)
$3F01:  BG Palette 0, color 1
$3F02:  BG Palette 0, color 2
$3F03:  BG Palette 0, color 3
$3F04:  BG Palette 1, color 1
...
$3F0F:  BG Palette 3, color 3
$3F10:  Mirror of $3F00
$3F11:  Sprite Palette 0, color 1
...
$3F1F:  Sprite Palette 3, color 3
```

x-nes handles the mirroring:

```rust
let i = (addr & 0x1F) as usize;
let i = if i & 0x13 == 0x10 { i & 0x0F } else { i };
// $10→$00, $14→$04, $18→$08, $1C→$0C
self.palette[i]
```

## 4.2 PPU Timing

The PPU generates a video frame as a sequence of scanlines. Each scanline has 341 dot cycles.

### 4.2.1 Frame Structure

```
┌─────────────────────────────────────────────────────────┐
│ Scanlines 0-239:  Visible picture (240 scanlines)       │
│   Cycles 1-256:   Background tile fetch + pixel render  │
│   Cycle 257:      Horizontal scroll copy + sprite eval  │
│   Cycles 258-340: Idle (no rendering)                   │
├─────────────────────────────────────────────────────────┤
│ Scanline 240:     Post-render (idle)                    │
├─────────────────────────────────────────────────────────┤
│ Scanlines 241-260: VBlank (vertical blanking interval)  │
│   Scanline 241, cycle 1: Set VBlank flag, trigger NMI   │
├─────────────────────────────────────────────────────────┤
│ Scanline 261:     Pre-render line (idle)                │
│   Cycle 1: Clear VBlank/sprite flags                    │
│   Cycle 0 (start): Vertical scroll copy                 │
│   Cycle 340: Skip on odd frames (rendering enabled)     │
└─────────────────────────────────────────────────────────┘
Total: 262 scanlines × 341 cycles = 89,342 PPU dots per frame
```

### 4.2.2 Rendering Per Scanline

During visible scanlines, the PPU uses an **on-the-fly renderer** that computes each pixel independently from scroll state snapshots:

1. **Cycle 1 (and every 8th cycle):** Fetch background tile pattern, increment coarse X
2. **Cycles 1-256:** Render one pixel per cycle
3. **Cycle 256:** Increment coarse Y (vertical scroll)
4. **Cycle 257:** Copy horizontal scroll bits from `t` to `v`, evaluate sprites for next scanline

### 4.2.3 Scroll State Snapshots

The on-the-fly renderer uses `render_v` and `render_fine_x` — snapshots taken at prerender cycle 0 and updated at cycle 257 of each scanline:

```rust
// Prerender cycle 0:
self.render_v = self.v;
self.render_fine_x = self.fine_x;

// Cycle 257:
self.render_v = (self.render_v & !V_HORIZONTAL_MASK) | (self.v & V_HORIZONTAL_MASK);
self.render_fine_x = self.fine_x;
```

## 4.3 PPU Registers

### 4.3.1 PPUCTRL ($2000) — Write Only

```
Bit 7: NMI on VBlank (V)
Bit 6: PPU master/slave (P) — ignored by most emulators
Bit 5: Sprite size (H) — 0=8×8, 1=8×16
Bit 4: Background pattern table (B) — 0=$0000, 1=$1000
Bit 3: Sprite pattern table (S) — 0=$0000, 1=$1000
Bit 2: VRAM address increment (I) — 0=+1, 1=+32
Bits 1-0: Base nametable (NN) — 0-3 for $2000/$2400/$2800/$2C00
```

Writing $2000 triggers NMI edge detection. If NMI was disabled and is now enabled while VBlank is active, the NMI fires immediately.

### 4.3.2 PPUMASK ($2001) — Write Only

```
Bit 7: Emphasize blue
Bit 6: Emphasize green
Bit 5: Emphasize red
Bit 4: Show sprites (s)
Bit 3: Show background (b)
Bit 2: Show sprites in left 8 columns (M)
Bit 1: Show background in left 8 columns (m)
Bit 0: Greyscale (G)
```

### 4.3.3 PPUSTATUS ($2002) — Read Only

```
Bit 7: VBlank flag (V)
Bit 6: Sprite 0 hit (S)
Bit 5: Sprite overflow (O)
Bits 4-0: Open bus (decayed)
```

Reading $2002 clears bit 7, resets the write toggle (`w`), and the low 5 bits return the open bus value with decay. If read on the same cycle that VBlank would be set (scanline 241, cycle 1), VBlank is **suppressed**.

```rust
pub fn read_status(&mut self) -> u8 {
    let s = self.status;
    self.status &= !0x80;  // clear VBlank flag
    self.w = 0;            // reset write toggle
    let result = (s & 0xE0) | (self.get_open_bus() & 0x1F);
    // VBlank suppression edge case
    if self.scanline == VBLANK_START && self.cycle == 1 && (s & 0x80) == 0 {
        self.vbl_suppressed = true;
    }
    self.update_nmi_edge(false);
    result
}
```

### 4.3.4 PPUADDR ($2006) and PPUDATA ($2007)

These work together to provide random access to PPU memory:

```rust
pub fn write_addr(&mut self, val: u8) {
    if self.w == 0 {
        // First write: high byte (upper 6 bits to t)
        self.t = ((self.t & 0x00FF) | ((val as u16) << 8)) & 0x3FFF;
        self.w = 1;
    } else {
        // Second write: low byte, then t→v
        self.t = (self.t & 0xFF00) | val as u16;
        self.v = self.t;
        self.w = 0;
    }
}

pub fn read_data(&mut self, mapper: &mut Mapper) -> u8 {
    let addr = self.v & 0x3FFF;
    let val = if addr < 0x2000 {
        self.chr_read(addr, mapper)         // CHR read via mapper
    } else {
        self.ppu_read_nt(addr, mapper)      // Nametable/palette read
    };
    let result = if addr < 0x3F00 {
        self.data_buffer                     // Buffer: 1-cycle delay
    } else {
        (self.get_open_bus() & 0xC0) | (val & 0x3F)  // Palette: immediate + open bus
    };
    // Update buffer for next read
    if addr < 0x3F00 {
        self.data_buffer = val;
    } else {
        self.data_buffer = self.ppu_read_nt(addr & 0x2FFF, mapper);
    }
    self.v = self.v.wrapping_add(if self.ctrl & 0x04 != 0 { 32 } else { 1 });
    result
}
```

### 4.3.5 PPUSCROLL ($2005) — Write Only

Two writes (toggled by `w`):

```rust
pub fn write_scroll(&mut self, val: u8) {
    if self.w == 0 {
        // First write: coarse X (val >> 3) + fine X (val & 7)
        self.t = (self.t & 0xFFE0) | ((val >> 3) as u16);
        self.fine_x = val & 7;
        self.w = 1;
    } else {
        // Second write: coarse Y + fine Y + nametable Y
        self.t = (self.t & 0xFC1F)
            | (((val as u16) & 0x07) << 12)   // fine Y → bits 12-14
            | (((val as u16) & 0xF8) << 2);   // coarse Y → bits 5-9
        self.w = 0;
    }
}
```

### 4.3.6 OAMADDR ($2003) and OAMDATA ($2004)

OAM (Object Attribute Memory) holds 64 sprites at 4 bytes each (Y, tile, attr, X):

```rust
pub fn write_oam_addr(&mut self, val: u8) { self.oam_addr = val; }
pub fn write_oam_data(&mut self, val: u8) {
    self.oam[self.oam_addr as usize] = val;
    self.oam_addr = self.oam_addr.wrapping_add(1);
}
pub fn read_oam_data(&mut self) -> u8 {
    let mut val = self.oam[self.oam_addr as usize];
    if self.oam_addr & 0x03 == 2 { val &= 0xE3; } // Bits 2-4 read as 0
    val
}
```

## 4.4 On-the-Fly Rendering

x-nes uses an on-the-fly pixel renderer that computes each pixel independently rather than emulating the PPU's internal shift registers. This gives more flexibility and simplifies MMC5 extended attribute support.

### 4.4.1 Background Pixel Computation

```rust
fn compute_bg_pixel(&mut self, x: u16, y: u16, mapper: &mut Mapper) -> (u8, u8) {
    // 1. Extract scroll state from render_v snapshot
    let coarse_x = self.render_v & 0x001F;
    let coarse_y = (self.render_v >> 5) & 0x001F;
    let fine_y = (self.render_v >> 12) & 0x0007;
    let nt = (self.render_v >> 10) & 0x0003;

    // 2. Compute world coordinates
    let world_x = (coarse_x << 3) + self.render_fine_x as u16 + x;
    let world_y = (coarse_y << 3) + fine_y + y;

    // 3. Handle nametable wrapping
    let mut actual_nt = nt;
    if (world_x >> 8) & 1 != 0 { actual_nt ^= 1; }    // Horizontal wrap
    if ((world_y >> 3) / 30) & 1 != 0 { actual_nt ^= 2; } // Vertical wrap

    // 4. Compute tile position
    let tile_x = (world_x >> 3) & 31;
    let tile_y = ((world_y >> 3) % 30) & 31;
    let pixel_x = world_x & 7;
    let pixel_y = world_y & 7;

    // 5. Fetch tile index from nametable
    let nt_addr = 0x2000 | (actual_nt << 10) | (tile_y << 5) | tile_x;
    let tile_index = self.ppu_read_nt(nt_addr, mapper);

    // 6. MMC5 ExRAM mode 1: extended attributes
    let ex_ram_mode = mapper.get_ex_ram_mode();
    if ex_ram_mode == 1 {
        let exram_byte = mapper.read_ex_ram_byte(nt_addr & 0x03FF);
        mapper.set_extended_chr_bank(exram_byte);
        mapper.set_chr_fetch_bg();
        let tile_addr = ((tile_index as u16) << 4) | pixel_y;
        let low = mapper.ppu_read(tile_addr);
        let high = mapper.ppu_read(tile_addr | 0x0008);
        let pixel = ((high >> (7-pixel_x)) & 1) << 1 | ((low >> (7-pixel_x)) & 1);
        if pixel == 0 { return (0, self.palette[0]); }
        let pal_group = (exram_byte >> 6) & 3;
        return (pixel, self.palette[((pal_group as usize) << 2) | pixel as usize]);
    }

    // 7. Standard rendering: fetch tile data + attribute
    mapper.set_chr_fetch_bg();
    let bg_table = if self.ctrl & 0x10 != 0 { 0x1000 } else { 0x0000 };
    let tile_addr = bg_table | ((tile_index as u16) << 4) | pixel_y;
    let low = mapper.ppu_read(tile_addr);
    let high = mapper.ppu_read(tile_addr | 0x0008);
    let pixel = ((high >> (7-pixel_x)) & 1) << 1 | ((low >> (7-pixel_x)) & 1);

    if pixel == 0 {
        (0, self.palette[0])
    } else {
        // Fetch attribute for palette group
        let attr_addr = nt_base | 0x03C0 | ((tile_y >> 2) << 3) | (tile_x >> 2);
        let attr = self.ppu_read_nt(attr_addr, mapper);
        let pal_group = (attr >> (((tile_x>>1)&1)<<1 | ((tile_y>>1)&1)<<2)) & 3;
        (pixel, self.palette[((pal_group as usize) << 2) | pixel as usize])
    }
}
```

### 4.4.2 Sprite Rendering

Sprites are evaluated once per scanline (at cycle 257 for the next line). The evaluator scans OAM and selects up to 8 sprites that intersect the next scanline:

```rust
fn evaluate_sprites_for(&mut self, sl: u16, mapper: &mut Mapper) {
    self.sprite_count = 0;
    let sprite_h = if self.ctrl & 0x20 != 0 { 16 } else { 8 };
    for i in (0..0x100).step_by(4) {
        let sy = self.oam[i] as u16;
        if sy <= sl && sl < sy + sprite_h {
            if self.sprite_count < 8 {
                self.sprite_indices[self.sprite_count as usize] = (i >> 2) as u8;
                self.sprite_count += 1;
            } else {
                self.status |= 0x20; // Sprite overflow
            }
        }
    }
    // Fetch sprite patterns for all 8 slots (including dummy fetches for unused)
    mapper.set_chr_fetch_sprite();
    for si in 0..8 { /* pattern fetch for each slot */ }
}
```

Sprite rendering per pixel checks if any evaluated sprite covers this X position, handles priority (behind background), sprite 0 hit detection, and 8×16 vs 8×8 sprite size.

## 4.5 MMC5 Integration Points

The PPU has specific hooks for MMC5 features:

| Hook | Purpose |
|------|---------|
| `nt_mapping()` | Returns MMC5's `$5105` value for custom NT routing |
| `read_nt_ext()` / `write_nt_ext()` | Handle ExRAM and fill mode NT sources (2 and 3) |
| `set_chr_fetch_bg()` / `set_chr_fetch_sprite()` | Tell MMC5 whether current PPU read is BG or sprite |
| `set_extended_chr_bank()` | Set ExRAM mode 1 per-tile CHR bank |
| `get_ex_ram_mode()` | Check if ExRAM mode 1 is active |
| `read_ex_ram_byte()` | Read ExRAM byte for extended attributes |

## 4.6 NMI Edge Detection

The PPU detects rising edges on the NMI enable signal (VBlank AND NMI-enabled). It tracks:
- `nmi_output`: previous NMI line state
- `nmi_latched`: edge detected, pending CPU sample (penultimate cycle)
- `nmi_from_vblank`: edge was from VBlank start (fires immediately)
- `nmi_deferred_pending`: NMI from $2000 write deferred to next instruction

```rust
pub fn update_nmi_edge(&mut self, from_vblank: bool) {
    let new_output = (self.status & STATUS_VBLANK != 0) && (self.ctrl & CTRL_NMI_ENABLE != 0);
    let edge = !self.nmi_output && new_output;
    if edge {
        self.nmi_latched = true;
        if from_vblank { self.nmi_from_vblank = true; }
    }
    self.nmi_output = new_output;
}
```

## Summary

- The PPU has its own 16KB memory with pattern tables, name tables, and palettes
- A frame is 262 scanlines × 341 cycles = 89,342 dots (~16.67ms at 60Hz)
- 240 scanlines are visible, 241-261 are VBlank + prerender
- NMI triggers at the start of VBlank (scanline 241, cycle 1)
- The on-the-fly renderer computes each pixel from scroll state snapshots
- MMC5 extended attributes add per-tile CHR bank and palette selection
- The write toggle (`w`) alternates between first/second writes for $2005/$2006
- $2007 reads have a 1-cycle buffer delay (except palettes)
