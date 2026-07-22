/// NES emulator debugging utilities.
///
/// Usage:
/// ```
/// use nes::debug::*;
/// save_frame_ppm(&bus.ppu.frame, "frame.ppm");
/// dump_ppu_state(&bus.ppu);
/// ```
use crate::mapper::Mapper;
use crate::ppu::Ppu;

// NES standard palette (index → RGB)
pub static PALETTE: [[u8; 3]; 64] = [
    [84, 84, 84],
    [0, 30, 116],
    [8, 0, 144],
    [68, 0, 136],
    [124, 0, 60],
    [164, 0, 28],
    [168, 0, 0],
    [136, 0, 0],
    [92, 40, 0],
    [40, 68, 0],
    [0, 84, 0],
    [0, 80, 48],
    [0, 68, 100],
    [0, 0, 0],
    [0, 0, 0],
    [0, 0, 0],
    [180, 180, 180],
    [12, 84, 196],
    [48, 60, 216],
    [116, 44, 196],
    [172, 24, 152],
    [216, 0, 76],
    [220, 8, 0],
    [188, 48, 0],
    [128, 80, 0],
    [72, 104, 0],
    [16, 120, 0],
    [0, 116, 68],
    [0, 104, 108],
    [0, 0, 0],
    [0, 0, 0],
    [0, 0, 0],
    [252, 252, 252],
    [100, 176, 252],
    [144, 144, 252],
    [200, 124, 252],
    [252, 116, 252],
    [252, 116, 184],
    [252, 120, 112],
    [252, 152, 56],
    [240, 184, 0],
    [188, 208, 0],
    [132, 220, 48],
    [88, 216, 120],
    [68, 208, 168],
    [0, 0, 0],
    [0, 0, 0],
    [0, 0, 0],
    [252, 252, 252],
    [192, 228, 252],
    [208, 212, 252],
    [232, 204, 252],
    [252, 200, 252],
    [252, 196, 224],
    [252, 200, 184],
    [252, 212, 160],
    [252, 224, 144],
    [228, 236, 136],
    [200, 240, 144],
    [168, 240, 176],
    [176, 236, 200],
    [0, 0, 0],
    [0, 0, 0],
    [0, 0, 0],
];

// ─── Frame Capture ─────────────────────────────────────────────────

/// Save the current frame to a PPM file (viewable in any image viewer).
pub fn save_frame_ppm(frame: &[u8; 61440], path: &str) {
    use std::io::Write;
    let mut f = std::fs::File::create(path).unwrap();
    writeln!(f, "P6\n256 240\n255").unwrap();
    for y in 0..240u16 {
        for x in 0..256u16 {
            let idx = frame[(y as usize) * 256 + (x as usize)] as usize & 0x3F;
            let rgb = PALETTE[idx];
            f.write_all(&rgb).unwrap();
        }
    }
}

/// Compare two frames pixel-by-pixel, return first diff (x, y) or None.
pub fn frame_diff(a: &[u8; 61440], b: &[u8; 61440]) -> Option<(u16, u16)> {
    for i in 0..61440 {
        if a[i] != b[i] {
            return Some(((i % 256) as u16, (i / 256) as u16));
        }
    }
    None
}

// ─── PPU State Dump ─────────────────────────────────────────────────

/// Print full PPU state to stderr.
pub fn dump_ppu_state(ppu: &Ppu) {
    eprintln!("═══ PPU STATE ═══");
    eprintln!(
        "Position: scanline={}/261  cycle={}/340  tick={}",
        ppu.scanline, ppu.cycle, ppu.tick_count
    );
    eprintln!(
        "Registers: CTRL=${:02X}  MASK=${:02X}  STATUS=${:02X}",
        ppu.ctrl, ppu.mask, ppu.status
    );
    if ppu.mask & 0x18 == 0 {
        eprintln!("  ⚠ RENDERING DISABLED (both BG and sprites off)");
    }
    eprintln!(
        "Scroll:    v=${:04X}  t=${:04X}  fine_x={}  w={}",
        ppu.v, ppu.t, ppu.fine_x, ppu.w
    );
    eprintln!(
        "NMI: output={} latched={} vblank={} deferred={}",
        ppu.nmi_output as u8,
        ppu.nmi_latched as u8,
        ppu.nmi_from_vblank as u8,
        ppu.nmi_deferred_pending as u8
    );
    eprintln!(
        "Frame: complete={} odd={}",
        ppu.frame_complete as u8, ppu.odd_frame as u8
    );
    eprintln!(
        "Render: render_v=${:04X}  render_fine_x={}",
        ppu.render_v, ppu.render_fine_x
    );
}

/// Print the palette table to stderr.
pub fn dump_palette(ppu: &Ppu) {
    eprintln!("═══ PALETTE ═══");
    for pal in 0..4 {
        eprint!("BG pal {pal}: ");
        for c in 0..4 {
            eprint!("${:02X} ", ppu.palette[pal * 4 + c]);
        }
        eprintln!();
    }
    for pal in 0..4 {
        eprint!("SP pal {pal}: ");
        for c in 0..4 {
            eprint!("${:02X} ", ppu.palette[0x10 + pal * 4 + c]);
        }
        eprintln!();
    }
    if ppu.palette[0..8].iter().all(|&p| p == 0x0F) {
        eprintln!("  ⚠ ALL BLACK ($0F) — palette may not be loaded");
    }
}

/// Dump OAM (sprite memory) — only shows sprites that would be visible.
pub fn dump_oam(ppu: &Ppu) {
    eprintln!("═══ OAM (Sprite Memory) ═══");
    let mut count = 0;
    for i in (0..256).step_by(4) {
        let y = ppu.oam[i];
        if y < 0xEF {
            let tile = ppu.oam[i + 1];
            let attr = ppu.oam[i + 2];
            let x = ppu.oam[i + 3];
            eprintln!(
                "  #{}: ({:>3},{:>3}) tile=${:02X} palette={} {} {}",
                i / 4,
                x,
                y,
                tile,
                attr & 3,
                if attr & 0x20 != 0 {
                    "[behind]"
                } else {
                    "[front]"
                },
                if attr & 0x80 != 0 { "[vflip]" } else { "" }
            );
            count += 1;
        }
    }
    if count == 0 {
        eprintln!("  (no sprites)");
    } else {
        eprintln!("  Total: {count} sprites");
    }
}

/// Dump nametable tile data (32x30 grid).
pub fn dump_nametable(vram: &[u8; 0x1000], nt: u16) {
    eprintln!("═══ Nametable {nt} ═══");
    let base = (nt as usize) * 0x400;
    for row in 0..30 {
        eprint!("{row:>2}: ");
        for col in 0..32 {
            let tile = vram[base + row * 32 + col];
            if tile == 0 {
                eprint!(".. ");
            } else {
                eprint!("{tile:02X} ");
            }
        }
        eprintln!();
    }
}

/// Dump attribute table bytes.
pub fn dump_attribute_table(vram: &[u8; 0x1000], nt: u16) {
    eprintln!("═══ Attr Table {nt} ═══");
    let base = (nt as usize) * 0x400 + 0x3C0;
    for row in 0..8 {
        eprint!("  ");
        for col in 0..8 {
            eprint!("{:02X} ", vram[base + row * 8 + col]);
        }
        eprintln!();
    }
}

/// Show tiles from CHR data as ASCII art.
pub fn dump_chr_page(chr: &[u8], page_base: usize, first_tile: u16, count: u16) {
    eprintln!(
        "═══ CHR at ${:04X} tiles {}-{} ═══",
        page_base,
        first_tile,
        first_tile + count - 1
    );
    for t in first_tile..first_tile + count {
        let base = page_base + (t as usize) * 16;
        if base + 16 > chr.len() {
            eprintln!("  (beyond CHR size)");
            break;
        }
        eprint!("Tile {t:>3}: ");
        for y in 0..8 {
            let lo = chr[base + y];
            let hi = chr[base + y + 8];
            for x in (0..8).rev() {
                let pixel = ((hi >> x) & 1) << 1 | ((lo >> x) & 1);
                eprint!("{}", ['.', ':', 'o', 'O'][pixel as usize]);
            }
            if y < 7 {
                eprint!(" ");
            }
        }
        eprintln!();
    }
}

// ─── Quick Diagnostics ──────────────────────────────────────────────

/// Quick diagnostic: check common failure modes.
pub fn quick_diagnostics(ppu: &Ppu, mapper: &Mapper, rom_name: &str) {
    eprintln!("═══ DIAGNOSTICS for {rom_name} ═══");

    let bg = ppu.mask & 0x08 != 0;
    let sp = ppu.mask & 0x10 != 0;
    eprint!(
        "[1] Render: BG={} SP={} ",
        if bg { "ON " } else { "OFF" },
        if sp { "ON " } else { "OFF" }
    );
    if !bg && !sp {
        eprintln!("⚠ NOTHING RENDERING");
    } else {
        eprintln!("✓");
    }

    let has_palette = ppu.palette[0..8].iter().any(|&p| p != 0x0F && p != 0x00);
    eprintln!(
        "[2] Palette: {}",
        if has_palette {
            "✓ loaded"
        } else {
            "⚠ ALL BLACK"
        }
    );

    let tile_count = ppu.vram.iter().take(0x800).filter(|&&t| t != 0).count();
    eprintln!(
        "[3] Nametable: {} / 2048 non-zero{}",
        tile_count,
        if tile_count == 0 { " ⚠ EMPTY" } else { "" }
    );

    let sprite_count = ppu.oam.iter().step_by(4).filter(|&&y| y < 0xEF).count();
    eprintln!("[4] Sprites: {sprite_count}");

    eprintln!("[5] Mapper: {}", mapper_type_name(mapper));

    let frame = ppu.tick_count / 89342;
    eprintln!("[6] Frame: ≈{} ({} PPU ticks)", frame, ppu.tick_count);
    eprintln!();
}

fn mapper_type_name(mapper: &Mapper) -> &'static str {
    match mapper {
        Mapper::Nrom(_) => "NROM (0)",
        Mapper::UxRom(_) => "UxROM (2)",
        Mapper::Cnrom(_) => "CNROM (3)",
        Mapper::Mmc1(_) => "MMC1 (1)",
        Mapper::Mmc2(_) => "MMC2 (9)",
        Mapper::Mmc3(_) => "MMC3 (4)",
        Mapper::Mmc4(_) => "MMC4 (10)",
        Mapper::Mmc5(_) => "MMC5 (5)",
        Mapper::Axrom(_) => "AxROM (7)",
        Mapper::Gxrom(_) => "GxROM (66)",
        Mapper::Null => "NULL",
    }
}

// ─── MMC5-Specific Dumps ───────────────────────────────────────────

/// Dump MMC5 internal register state (if mapper is MMC5).
pub fn dump_mmc5_state(mapper: &Mapper) {
    if let Mapper::Mmc5(m) = mapper {
        eprintln!("═══ MMC5 Registers ═══");
        eprintln!("PRG mode={} CHR mode={}", m.prg_mode, m.chr_mode);
        eprintln!(
            "PRG banks: ${:02X} ${:02X} ${:02X} ${:02X}",
            m.prg_reg[0], m.prg_reg[1], m.prg_reg[2], m.prg_reg[3]
        );
        eprintln!(
            "CHR sprite banks: ${:02X} ${:02X} ${:02X} ${:02X} ${:02X} ${:02X} ${:02X} ${:02X}",
            m.chr_sprite_reg[0],
            m.chr_sprite_reg[1],
            m.chr_sprite_reg[2],
            m.chr_sprite_reg[3],
            m.chr_sprite_reg[4],
            m.chr_sprite_reg[5],
            m.chr_sprite_reg[6],
            m.chr_sprite_reg[7]
        );
        eprintln!(
            "CHR BG banks: ${:02X} ${:02X} ${:02X} ${:02X}",
            m.chr_bg_reg[0], m.chr_bg_reg[1], m.chr_bg_reg[2], m.chr_bg_reg[3]
        );
        eprintln!(
            "CHR upper bits={} ExRAM mode={} NT mapping=${:02X}",
            m.chr_upper_bits, m.ex_ram_mode, m.nt_mapping_reg
        );
        eprintln!(
            "PRG RAM protect: ${:02X} ${:02X} → {}",
            m.prg_ram_protect1,
            m.prg_ram_protect2,
            if m.prg_ram_protect1 == 0x02 && m.prg_ram_protect2 == 0x01 {
                "PROTECTED"
            } else {
                "unlocked"
            }
        );
        eprintln!(
            "IRQ: scanline={} enabled={} pending={} status=${:02X}",
            m.irq_scanline, m.irq_enable as u8, m.irq_pending_flag as u8, m.irq_status
        );
        eprintln!("Fill: tile=${:02X} attr={}", m.fill_tile, m.fill_attr);
        eprintln!("Multiplier: {} × {} = {}", m.mul_a, m.mul_b, m.mul_result);
    } else {
        eprintln!("(mapper is not MMC5)");
    }
}

/// Dump first N bytes of MMC5 `ExRAM`.
pub fn dump_mmc5_exram(mapper: &Mapper, count: usize) {
    if let Mapper::Mmc5(m) = mapper {
        eprintln!("═══ MMC5 ExRAM (first {count} bytes) ═══");
        for row in 0..count.div_ceil(16) {
            eprint!("{:03X}: ", row * 16);
            for col in 0..16 {
                let i = row * 16 + col;
                if i < count {
                    let b = m.ex_ram[i];
                    if b == 0 {
                        eprint!(".. ");
                    } else {
                        eprint!("{b:02X} ");
                    }
                }
            }
            eprintln!();
        }
    } else {
        eprintln!("(mapper is not MMC5)");
    }
}

// ─── Scanline / Pixel Inspection ───────────────────────────────────

/// Print what tile and palette are active at a given (x,y) pixel position.
pub fn inspect_pixel(ppu: &Ppu, x: u16, y: u16) {
    let color_idx = ppu.frame[(y as usize) * 256 + (x as usize)];
    let render_v = ppu.render_v;
    let coarse_x = render_v & 0x001F;
    let coarse_y = (render_v >> 5) & 0x001F;
    let fine_y = (render_v >> 12) & 0x0007;
    let nt = (render_v >> 10) & 0x0003;

    let world_x = (coarse_x << 3) + ppu.render_fine_x as u16 + x;
    let world_y = (coarse_y << 3) + fine_y + y;
    let mut actual_nt = nt;
    if (world_x >> 8) & 1 != 0 {
        actual_nt ^= 1;
    }
    if ((world_y >> 3) / 30) & 1 != 0 {
        actual_nt ^= 2;
    }

    let tile_x = (world_x >> 3) & 31;
    let tile_y = ((world_y >> 3) % 30) & 31;

    let nt_addr = 0x2000 | (actual_nt << 10) | (tile_y << 5) | tile_x;
    let vram_base = (actual_nt as usize & 1) * 0x400;
    let tile_index = ppu.vram[vram_base + (tile_y as usize * 32 + tile_x as usize)];

    let attr_addr = 0x2000 | (actual_nt << 10) | 0x03C0 | ((tile_y / 4) << 3) | (tile_x / 4);
    let attr_idx = vram_base + 0x3C0 + ((tile_y as usize / 4) * 8 + tile_x as usize / 4);
    let attr = ppu.vram[attr_idx];
    let pal_shift = (((tile_x >> 1) & 1) << 1) | (((tile_y >> 1) & 1) << 2);
    let pal_group = (attr >> pal_shift) & 3;

    eprintln!("═══ Pixel ({x},{y}) ═══");
    eprintln!(
        "Color index: ${:02X} → RGB({},{},{})",
        color_idx,
        PALETTE[(color_idx & 0x3F) as usize][0],
        PALETTE[(color_idx & 0x3F) as usize][1],
        PALETTE[(color_idx & 0x3F) as usize][2]
    );
    eprintln!("World: ({world_x},{world_y})  NT={actual_nt}  Tile: ({tile_x},{tile_y})");
    eprintln!("NT addr: ${nt_addr:04X}  Tile index: ${tile_index:02X}");
    eprintln!("Attr addr: ${attr_addr:04X}  Attr byte: ${attr:02X}  Palette group: {pal_group}");
    if color_idx != 0 {
        eprintln!(
            "Palette color: ${:02X} (pal {} color {})",
            ppu.palette[(pal_group as usize) * 4 + (color_idx as usize & 3)],
            pal_group,
            color_idx & 3
        );
    }
}

// ─── Trace Logging ─────────────────────────────────────────────────

/// Simple tracing: log PPU writes/reads to stderr.
/// Call this from your register handlers to trace PPU activity.
pub fn trace_ppu_reg(addr: u16, val: u8, is_write: bool, scanline: u16, cycle: u16) {
    let rw = if is_write { "WR" } else { "RD" };
    eprintln!("PPU [sl={scanline:>3} cy={cycle:>3}] ${addr:04X} {rw} ${val:02X}");
}

/// Log mapper register writes (call from `cpu_write` of each mapper).
pub fn trace_mapper_write(name: &str, addr: u16, val: u8) {
    eprintln!("MAPPER {name} ${addr:04X} = ${val:02X}");
}
