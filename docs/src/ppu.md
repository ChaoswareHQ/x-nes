# 4. Picture Processing Unit

The PPU (Picture Processing Unit) generates the NES video signal. It runs at three times the CPU speed and has its own memory system for tiles, palettes, and sprites.

## 4.1 PPU Memory

The PPU has its own 16KB address space ($0000-$3FFF), separate from the CPU:

```
$0000-$0FFF   4KB     Pattern Table 0 (256 tiles, 16 bytes each)
$1000-$1FFF   4KB     Pattern Table 1 (256 tiles)
$2000-$23BF   960B    Name Table 0 (tile indices)
$23C0-$23FF   64B     Attribute Table 0 (palette assignments)
$2400-$27BF   960B    Name Table 1
$27C0-$27FF   64B     Attribute Table 1
$2800-$2BFF            Name Table 2 (mirror of 0)
$2C00-$2FFF            Name Table 3 (mirror of 1)
$3F00-$3F0F   16B     Background palette
$3F10-$3F1F   16B     Sprite palette (mirrors $3F00-$3F0F partially)
```

### 4.1.1 Pattern Tables

Tiles are 8x8 pixels, each pixel being 2 bits (selecting one of 4 colors from a palette). A tile takes 16 bytes: 8 bytes for the low bitplane, 8 bytes for the high bitplane.

```
Tile at $0000:
  $0000:  $7E  →  bitplane 0: 01111110
  $0001:  $FF  →  bitplane 0: 11111111
  $0002:  $DB  →  bitplane 0: 11011011
  ...
  $0008:  $00  →  bitplane 1: 00000000
  $0009:  $00  →  bitplane 1: 00000000
  $000A:  $24  →  bitplane 1: 00100100
  ...
```

A pixel's color is (bitplane1_bit << 1) | bitplane0_bit, giving values 0-3.

### 4.1.2 Name Tables

A name table is a 32x30 grid of tile indices (960 bytes). Each byte selects which tile to draw from the pattern tables. The PPU's current name table is selected by bits 1-0 of PPUCTRL.

### 4.1.3 Attribute Tables

Attribute tables assign palettes to 2x2 tile regions. Each byte controls 4 tile groups (2 bits per group), selecting one of 4 background palettes.

### 4.1.4 Palette RAM

The NES has 32 palette entries, but 16 are mirrors:

```
$3F00:  Background color (universal)
$3F01:  BG Palette 1, color 1
$3F02:  BG Palette 1, color 2
$3F03:  BG Palette 1, color 3
$3F04:  BG Palette 2, color 1
...
$3F10:  Mirror of $3F00
$3F11:  Sprite Palette 1, color 1
...
```

x-nes handles the mirroring:

```rust
let i = (addr & 0x1F) as usize;
let i = if i & 0x13 == 0x10 { i & 0x0F } else { i };
self.palette[i]
```

## 4.2 PPU Timing

The PPU generates a video frame as a sequence of scanlines. Each scanline has 341 dot cycles.

### 4.2.1 Frame Structure

```
┌─────────────────────────────────────────────────────────┐
│ Scanlines 0-239:  Visible picture (240 scanlines)       │
│   Cycles 1-256:   Render pixels                         │
│   Cycles 257-340: Idle (no rendering)                   │
├─────────────────────────────────────────────────────────┤
│ Scanline 240:     Post-render (idle)                    │
├─────────────────────────────────────────────────────────┤
│ Scanlines 241-260: Vblank (vertical blanking)           │
│   Scanline 241, cycle 1: Set vblank flag, trigger NMI   │
├─────────────────────────────────────────────────────────┤
│ Scanline 261:     Pre-render (idle)                     │
│   Cycle 1: Clear vblank flag                            │
└─────────────────────────────────────────────────────────┘
Total: 262 scanlines, 341 cycles each = 89,342 cycles per frame
```

### 4.2.2 The Tick Function

Each PPU tick simulates one dot cycle:

```rust
pub fn tick(&mut self) {
    let sl = self.scanline;
    let cy = self.cycle;

    // Visible scanlines: render pixels during cycles 1-256
    if sl < 240 {
        if sl == 0 && cy == 0 {
            self.cycle = 1;  // Scanline 0, cycle 0 is special
            return;
        }
        if cy < 256 {
            self.frame[(sl as usize) * 256 + (cy as usize)] = self.status >> 7;
        }
    }
    // Vblank start
    else if sl == 241 && cy == 1 {
        self.status |= 0xC0;  // Set vblank + sprite 0 hit
        if self.ctrl & 0x80 != 0 {
            self.nmi_pending = true;  // Trigger NMI if enabled
        }
    }
    // Pre-render: clear vblank
    else if sl == 261 && cy == 1 {
        self.status &= 0x3F;
    }

    // Advance to next cycle
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
```

The PPU is called 3 times per CPU cycle (the NES clock ratio). After each CPU instruction, x-nes advances the PPU by `cpu_cycles * 3` dots.

### 4.2.3 Batch Optimization

Most of the 341 cycles in a visible scanline don't do anything important — only cycles 1-256 render pixels, and cycles 257-340 are idle. When we know we're in the idle region, we can skip ahead:

```rust
pub fn tick_batch(&mut self, mut count: u16) {
    while count > 0 {
        if self.scanline < 240 && self.cycle >= 257 && self.cycle <= 340 {
            let skip = (341 - self.cycle).min(count);
            self.cycle += skip;
            count -= skip;
            continue;
        }
        self.tick();
        count -= 1;
    }
}
```

This reduces the number of individual tick calls by about 25%.

## 4.3 PPU Registers

### 4.3.1 PPUCTRL ($2000) — Write Only

```
Bit 7: NMI on vblank
Bit 6: PPU master/slave
Bit 5: Sprite size (0=8x8, 1=8x16)
Bit 4: Background pattern table address
Bit 3: Sprite pattern table address
Bit 2: VRAM address increment (0=+1, 1=+32)
Bits 1-0: Name table select (0-3)
```

When written, the name table select bits are immediately copied to the internal `t` register for scrolling.

### 4.3.2 PPUSTATUS ($2002) — Read Only

```
Bit 7: Vblank flag
Bit 6: Sprite 0 hit
Bit 5: Sprite overflow
```

Reading this register clears bit 7 (vblank) and the write toggle for $2005/$2006.

```rust
pub fn read_status(&mut self) -> u8 {
    let s = self.status;
    self.status &= 0x7F;  // clear vblank flag
    self.w = 0;           // reset write toggle
    s
}
```

### 4.3.3 PPUADDR ($2006) and PPUDATA ($2007)

These work together to provide random access to PPU memory. Writing to $2006 sets the address (two writes: high byte, then low byte). Reading or writing $2007 accesses the data at that address, then advances it.

```rust
pub fn write_addr(&mut self, val: u8) {
    if self.w == 0 {
        // First write: high byte
        self.t = ((self.t & 0x00FF) | ((val as u16) << 8)) & 0x3FFF;
        self.w = 1;
    } else {
        // Second write: low byte
        self.t = (self.t & 0xFF00) | val as u16;
        self.v = self.t;  // transfer to current address
        self.w = 0;       // reset toggle
    }
}

pub fn write_data(&mut self, val: u8) {
    let addr = self.v & 0x3FFF;
    self.ppu_write(addr, val);
    self.v = self.v.wrapping_add(if self.ctrl & 0x04 != 0 { 32 } else { 1 });
}
```

### 4.3.4 PPUSCROLL ($2005) — Write Only

Two writes set the scroll position: first X (divided by 8) + fine X, then Y (divided by 8) + fine Y. The write toggle (`w`) alternates between X and Y.

```rust
pub fn write_scroll(&mut self, val: u8) {
    if self.w == 0 {
        // First write: coarse X + fine X
        self.t = (self.t & 0xFFE0) | ((val >> 3) as u16);
        self.fine_x = val & 7;
        self.w = 1;
    } else {
        // Second write: coarse Y + fine Y
        self.t = (self.t & 0xFC1F) |
            (((val as u16) & 7) << 12) |
            (((val as u16) & 0xF8) << 2);
        self.w = 0;
    }
}
```

## 4.4 PPU Read Pipeline

Reading $2007 has a one-cycle delay: the PPU returns the value from its internal buffer (filled during the previous read), while the newly-read value goes into the buffer for the next read. Palette reads are an exception — they bypass the buffer and return the actual value.

```rust
pub fn read_data(&mut self) -> u8 {
    let addr = self.v & 0x3FFF;
    let val = self.ppu_read(addr);
    let result = if addr < 0x3F00 {
        self.data_buffer  // return buffered value
    } else {
        val               // palette: return real value
    };
    if addr & 0x3F00 != 0x3F00 {
        self.data_buffer = val;  // buffer new value (not palette)
    }
    self.v = self.v.wrapping_add(if self.ctrl & 0x04 != 0 { 32 } else { 1 });
    result
}
```

## Summary

- The PPU has its own 16KB memory with pattern tables, name tables, and palettes
- A frame is 262 scanlines × 341 cycles = 89,342 dots
- 240 scanlines are visible, 22 are vblank
- NMI is triggered at the start of vblank
- The PPU runs 3× faster than the CPU
- VRAM access goes through a read pipeline with a 1-cycle buffer
