# Graphics Debugging

NES graphics bugs are notoriously hard to diagnose because a single PPU timing error can cascade into completely broken visuals. This guide covers systematic approaches to identifying and fixing rendering issues.

## Understanding NES Graphics Architecture

Quick refresher on how the PPU builds a frame:

```
For each scanline (0-239):
    For each cycle (1-256):
        1. Fetch nametable byte (tile index)
        2. Fetch attribute byte (palette group)
        3. Fetch pattern table low byte (bitplane 0)
        4. Fetch pattern table high byte (bitplane 1)
        5. Combine into 2-bit pixel value
        6. Look up color from palette

For each scanline (1-256 during visible):
    Evaluate sprites for NEXT scanline (cycle 257)
    Render sprites on current scanline
```

## Visual Symptom → Likely Cause Map

### Black screen / no output
| Likely Cause | What to Check |
|-------------|---------------|
| PPU not rendering | `mask & 0x08` (BG) or `mask & 0x10` (sprites) = 0? |
| Palette all zeros | Check `palette[0..32]` — all $0F? |
| CHR bank pointing to empty ROM | Verify CHR bank registers in mapper |
| Nametable filled with tile $00 | Dump nametable memory |
| Wrong bank at $C000-$FFFF | PRG register init — reset vector must be in last bank |

### Garbled graphics / wrong tiles
| Likely Cause | What to Check |
|-------------|---------------|
| Wrong CHR bank | Mapper CHR register writes correct? |
| Wrong BG pattern table | `ctrl & 0x10` selects $0000 or $1000 |
| CHR-RAM not written | Game writes to PPU $0000-$1FFF but mapper ignores? |
| NT mapping wrong (MMC5) | Check `$5105` value |

### Wrong colors / wrong palette
| Likely Cause | What to Check |
|-------------|---------------|
| Palette not loaded | Check writes to $3F00-$3F1F |
| Attribute table corrupt | Dump attribute memory at $23C0-$23FF |
| Palette mirroring wrong | `i & 0x13 == 0x10 → i & 0x0F` correct? |
| ExRAM mode 1 palette | ExRAM bits 6-7 used for palette, not attribute table |

### Screen scrolling / tearing / wrong position
| Likely Cause | What to Check |
|-------------|---------------|
| Scroll registers wrong | `fine_x`, `v`, `t` values during rendering |
| `copy_horizontal()` timing | Happens at cycle 257, not earlier/later |
| NMI timing wrong | NMI fires during VBlank, game writes scroll in NMI handler |
| `render_v` out of sync | Snapshot taken at prerender cycle 0 and cycle 257 |

### Split-screen / status bar broken (MMC5/MMC3 IRQ)
| Likely Cause | What to Check |
|-------------|---------------|
| IRQ fires at wrong scanline | Check `$5203` value, scanline counter accuracy |
| IRQ not firing | `irq_enable` true? Scanline counter matching? |
| IRQ acknowledged too early/late | Game reads `$5204` to clear. Check timing. |
| Scroll change in IRQ handler | Game writes `$2005`/$2006 during IRQ |

### Sprites missing / wrong position
| Likely Cause | What to Check |
|-------------|---------------|
| OAM DMA not working | `$4014` write triggers 256-byte copy to OAM |
| Sprite evaluation wrong | Check `sprite_count`, `sprite_indices` |
| Sprite 0 hit timing | `status & 0x40` set at correct pixel? |
| 8×16 sprite selection | `ctrl & 0x20` — uses different pattern table logic |

### Flickering / missing scanlines
| Likely Cause | What to Check |
|-------------|---------------|
| PPU cycle counting off | 341 cycles per line, 262 lines per frame |
| Odd frame skip | Cycle 340 of prerender line skipped on odd frames |
| Rendering enable mid-scanline | `mask` written during visible scanline? |

## Pixel-Level Debugging

### Dump a Scanline as Text

For a quick visual of what the PPU is drawing:

```rust
fn dump_scanline(ppu: &Ppu, scanline: u16, palette: &[u32; 64]) {
    print!("SL {:3}: ", scanline);
    for x in 0..256 {
        let color_idx = ppu.frame[(scanline as usize) * 256 + x];
        // Map color index to representative char
        let ch = match color_idx & 0x3F {
            0 => '.',    // background
            1..=15 => '#',  // BG colors
            16..=31 => 'S', // sprite colors
            _ => '?',
        };
        print!("{}", ch);
    }
    println!();
}
```

### Compare Frame Output

Save frames at specific intervals and compare:

```rust
// Save frame 0
if frame_count == 0 {
    save_ppm(&ppu.frame, "frame0.ppm", &PALETTE);
}
// Save frame 60
if frame_count == 60 {
    save_ppm(&ppu.frame, "frame60.ppm", &PALETTE);
}
```

Use an image diff tool to compare with Mesen's output frame-by-frame.

### Look at Specific Pixels

Examine what happened at a particular (x,y) coordinate:

```rust
fn inspect_pixel(ppu: &Ppu, x: u16, y: u16) {
    let idx = (y as usize) * 256 + (x as usize);
    let color = ppu.frame[idx] & 0x3F;

    // Reconstruct what the pixel should be
    let coarse_x = (ppu.render_v & 0x001F) as u16;
    let coarse_y = ((ppu.render_v >> 5) & 0x001F) as u16;
    let fine_y = (ppu.render_v >> 12) & 0x0007;
    let nt = (ppu.render_v >> 10) & 0x0003;

    let world_x = (coarse_x << 3) + ppu.render_fine_x as u16 + x;
    let world_y = (coarse_y << 3) + fine_y + y;

    let tile_x = (world_x >> 3) & 31;
    let tile_y = ((world_y >> 3) % 30) & 31;

    println!("Pixel ({},{}): color=${:02X}  tile=({},{})  NT={}",
        x, y, color, tile_x, tile_y, nt);
}
```

## Nametable Viewer

Dump the nametable contents to understand what the PPU sees:

```rust
fn dump_nametable(ppu: &Ppu, nt: u16) {
    // nt: 0-3 for each 1KB nametable
    let base = 0x2000 | (nt << 10);
    println!("--- Nametable {} ---", nt);

    // Tile indices (32×30)
    for row in 0..30 {
        for col in 0..32 {
            let addr = base | (row << 5) | col;
            // NOTE: you need access to mapper and ppu_read_nt for this
        }
    }

    // Attribute table (8×8 groups of 2×2 tiles)
    println!("--- Attributes ---");
    for row in 0..8 {
        for col in 0..8 {
            let addr = base | 0x03C0 | (row << 3) | col;
        }
    }
}
```

Better approach: dump nametable from `ppu.vram`:

```rust
fn dump_nametable_vram(ppu: &Ppu, nt: u16) {
    let base = (nt as usize & 1) * 0x400;
    println!("--- Nametable {} (from VRAM) ---", nt);
    for row in 0..30 {
        print!("{:2}: ", row);
        for col in 0..32 {
            let tile = ppu.vram[base + (row * 32 + col) as usize];
            print!("{:02X} ", tile);
        }
        println!();
    }
}
```

## Pattern Table Viewer

Visualize CHR pattern data to verify tiles are what you expect:

```rust
fn dump_tile(chr: &[u8], tile_idx: u16) {
    println!("--- Tile {} ---", tile_idx);
    let base = (tile_idx as usize) * 16;
    for y in 0..8 {
        let lo = chr[base + y];
        let hi = chr[base + y + 8];
        for x in (0..8).rev() {
            let bit = ((hi >> x) & 1) << 1 | ((lo >> x) & 1);
            print!("{}", match bit { 0=>'.', 1=>'1', 2=>'2', _=>'3' });
        }
        println!();
    }
}
```

## Common Graphics Bug Fixes

### Bug: Background tiles show wrong pattern

**Symptom:** Tiles appear from wrong pattern table or wrong CHR bank.

**Cause checklist:**
1. Is `ctrl & 0x10` selecting the correct pattern table?
2. For mappers: is the CHR bank register correct?
3. For MMC5 ExRAM mode 1: is `set_chr_fetch_bg()` called before PPU reads?
4. For MMC5: is `extended_chr_bank` set from correct ExRAM byte?

**Fix:** Add logging at PPU read time to see what bank is being used.

### Bug: Nametable shows garbage data

**Symptom:** Screen full of wrong tiles, or tiles from wrong area.

**Cause checklist:**
1. Did the game write nametable data? Check writes to $2000-$2FFF.
2. Is mirroring correct? Check `mirroring()` return value.
3. For MMC5: is `$5105` (nt_mapping_reg) correct?
4. For MMC5: is ExRAM (source 2) or fill mode (source 3) being used?

**Fix:** Dump `ppu.vram[0..0x800]` and verify tiles are what the game wrote.

### Bug: Palette colors are wrong

**Symptom:** Everything has a color tint, or specific colors are incorrect.

**Cause checklist:**
1. Has the game written palette data? Check writes to $3F00-$3F1F.
2. Is palette mirroring correct? Check `i & 0x13 == 0x10 → i & 0x0F`.
3. Are attribute table reads correct? Each 2-bit group selects palette 0-3.
4. For MMC5 ExRAM mode 1: is the palette coming from ExRAM bits 6-7?

**Fix:** Dump palette after the game has loaded it:
```rust
for i in 0..32 {
    println!("Palette {:02X}: {:02X}", i, ppu.palette[i]);
}
```

### Bug: Sprites not appearing

**Symptom:** No sprites visible, or sprites at wrong positions.

**Cause checklist:**
1. Is `mask & 0x10` (show sprites) set?
2. Did OAM DMA ($4014 write) execute?
3. Is `sprite_count > 0` after evaluation?
4. Are sprite indices correct?
5. For MMC5: is `set_chr_fetch_sprite()` called for sprite CHR reads?
6. Is sprite 0 hit detection interfering?

**Fix:** Dump OAM after DMA:
```rust
for i in (0..256).step_by(4) {
    let y = ppu.oam[i];
    let tile = ppu.oam[i+1];
    let attr = ppu.oam[i+2];
    let x = ppu.oam[i+3];
    if y < 0xEF {  // valid sprite
        println!("Sprite {}: ({},{}) tile={:02X} attr={:02X}",
            i/4, x, y, tile, attr);
    }
}
```

### Bug: Screen split at wrong position (MMC3/MMC5 IRQ)

**Symptom:** Status bar too high/low, or screen shake.

**Cause checklist:**
1. Is the IRQ counter/target correct? Check `$5203` (MMC5) or `$C000` (MMC3).
2. Is the IRQ firing at the right scanline? Add logging in `notify_scanline`.
3. After IRQ, does the game change scroll registers correctly?
4. For MMC3: is `copy_horizontal()` timing correct at cycle 257?

**Fix:** Log IRQ fires with scanline position:
```rust
// In notify_scanline or clock_scanline:
if irq_fired {
    eprintln!("IRQ at scanline {} (PPU cycle {})", scanline, ppu_cycle);
}
```

### Bug: ExRAM Mode 1 rendering incorrect (MMC5)

**Symptom:** Castlevania III backgrounds look wrong — wrong tiles or colors.

**Cause checklist:**
1. Is `ex_ram_mode == 1`? Check `$5104`.
2. Is ExRAM populated? Dump first 64 bytes of ExRAM.
3. Is `compute_bg_pixel` reading the ExRAM byte for each tile?
4. Is `ppu_read` using `extended_chr_bank` when `bg_fetch && ex_ram_mode == 1`?
5. Is the palette from ExRAM bits 6-7 being used (not the attribute table)?

**Fix:** Add verbose logging in `compute_bg_pixel` for ExRAM mode 1:
```rust
if ex_ram_mode == 1 {
    let exram_byte = mapper.read_ex_ram_byte(exram_offset);
    eprintln!("Tile ({},{}): ExRAM={:02X} → CHR page={} palette={}",
        tile_x, tile_y, exram_byte,
        exram_byte & 0x3F, (exram_byte >> 6) & 3);
}
```

## Frame Diffing

The fastest way to find graphics bugs is to compare frames at different points in execution:

1. Save frame N and frame N+1 to PPM files
2. Pixel-diff them to find when/where changes occur
3. At first differing pixel, inspect PPU state:
   - What tile is at that position?
   - What palette was selected?
   - What CHR bank is mapped?
   - What are the scroll register values?

```rust
fn compare_frames(a: &[u8; 61440], b: &[u8; 61440]) -> Option<(usize, usize)> {
    for i in 0..61440 {
        if a[i] != b[i] {
            let y = i / 256;
            let x = i % 256;
            return Some((x, y));
        }
    }
    None
}
```

## Pixel Color Reference

The NES palette (64 colors, but only 54 unique):

```
$00 Gray      $01 Blue      $02 Dark Blue $03 Purple   $04 Dark Purple $05 Dark Red  $06 Brown     $07 Dark Brown
$08 Olive     $09 Green     $0A Dark Green $0B Cyan     $0C Dark Cyan  $0D Blue-Gray $0E Dark Gray $0F Black
$10 Lt Gray   $11 Lt Blue   $12 Lt Blue 2 $13 Lt Purple $14 Lt Magenta  $15 Lt Red    $16 Yellow    $17 Gold
$18 Lt Olive  $19 Lt Green  $1A Lt Green2 $1B Lt Cyan   $1C Lt Cyan 2  $1D Lt Gray 2 $1E Lt Gray3 $1F White
$20-$3F: Mirrors of $00-$1F
```
