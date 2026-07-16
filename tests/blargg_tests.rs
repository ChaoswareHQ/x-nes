use std::fs;
use std::path::Path;

use nes::bus::Bus;
use nes::cpu::CpuRp2a03;
use nes::rom::Rom;
use nes::{reset, tick};

struct BlarggRunner {
    cpu: CpuRp2a03,
    bus: Bus,
    total_cycles: u64,
    total_frames: u32,
}

impl BlarggRunner {
    fn new(rom_path: &Path) -> Self {
        let data = fs::read(rom_path).expect("failed to read ROM");
        let rom = Rom::new(&data).expect("invalid iNES ROM");
        let mut cpu = CpuRp2a03::new(0);
        let mut bus = Bus::new(rom.create_mapper());
        reset(&mut cpu, &mut bus);
        Self {
            cpu,
            bus,
            total_cycles: 0,
            total_frames: 0,
        }
    }

    fn tick_frame(&mut self) {
        loop {
            let cycles = tick(&mut self.cpu, &mut self.bus);
            self.total_cycles += cycles as u64;
            if self.bus.ppu.frame_complete {
                self.bus.ppu.frame_complete = false;
                self.total_frames += 1;
                return;
            }
            if self.total_cycles > 200_000_000 {
                return;
            }
        }
    }

    fn run_frames(&mut self, n: u32) {
        for _ in 0..n {
            self.tick_frame();
            if self.total_cycles > 200_000_000 {
                break;
            }
        }
    }

    /// Read from $6000+ (test result area) through the bus
    fn read_result(&mut self, addr: u16) -> u8 {
        self.bus.read(addr)
    }

    /// Read text output from $6004
    fn read_text(&mut self) -> String {
        let mut s = String::new();
        for offset in 0..0x1FFC {
            let b = self.read_result(0x6004 + offset);
            if b == 0 {
                break;
            }
            s.push(b as char);
        }
        s
    }

    /// Check test status ($6000 with signature validation)
    fn status(&mut self) -> u8 {
        self.read_result(0x6000)
    }

    fn has_signature(&mut self) -> bool {
        self.read_result(0x6001) == 0xDE && self.read_result(0x6002) == 0xB0
    }
}

fn run_test(name: &str, rom_path: &str, max_frames: u32) {
    let path = Path::new(rom_path);
    if !path.exists() {
        println!("  SKIP: {} (ROM not found)", name);
        return;
    }

    let mut runner = BlarggRunner::new(path);
    runner.run_frames(max_frames);

    let has_sig = runner.has_signature();
    let status = runner.status();
    let text = runner.read_text();

    if has_sig {
        if status == 0 {
            println!("  PASS: {} ({} frames)", name, runner.total_frames);
        } else {
            println!(
                "  FAIL: {} (code={}, {} frames)",
                name, status, runner.total_frames
            );
            if !text.is_empty() {
                for line in text.lines() {
                    println!("    | {}", line);
                }
            }
            panic!("{} failed with code {}", name, status);
        }
    } else {
        if status == 0 {
            println!(
                "  PASS: {} (status=0, {} frames)",
                name, runner.total_frames
            );
        } else {
            println!(
                "  ?: {} (status={}, {} frames)",
                name, status, runner.total_frames
            );
        }
    }
}

// ========================================================================
// PPU Tests
// ========================================================================

#[test]
fn blargg_ppu_palette_ram() {
    run_test("palette_ram", "tests/roms/palette_ram.nes", 600);
}

#[test]
fn blargg_ppu_power_up_palette() {
    run_test("power_up_palette", "tests/roms/power_up_palette.nes", 600);
}

#[test]
fn blargg_ppu_sprite_ram() {
    run_test("sprite_ram", "tests/roms/sprite_ram.nes", 600);
}

#[test]
fn blargg_ppu_vbl_clear_time() {
    run_test("vbl_clear_time", "tests/roms/vbl_clear_time.nes", 600);
}

#[test]
fn blargg_ppu_vram_access() {
    run_test("vram_access", "tests/roms/vram_access.nes", 600);
}

#[test]
fn blargg_sprite_hit_basics() {
    run_test(
        "sprite_hit_01_basics",
        "tests/roms/spr_hit_01_basics.nes",
        600,
    );
}

#[test]
fn blargg_sprite_overflow_basics() {
    run_test(
        "sprite_overflow_01_basics",
        "tests/roms/spr_ovf_01_basics.nes",
        600,
    );
}

#[test]
fn blargg_ppu_vbl_nmi() {
    run_test("ppu_vbl_nmi", "tests/roms/ppu_vbl_nmi.nes", 1200);
}

#[test]
fn blargg_ppu_open_bus() {
    run_test("ppu_open_bus", "tests/roms/ppu_open_bus.nes", 1200);
}

#[test]
fn blargg_ppu_read_buffer() {
    run_test(
        "test_ppu_read_buffer",
        "tests/roms/test_ppu_read_buffer.nes",
        3000,
    );
}

// ========================================================================
// APU Tests
// ========================================================================

#[test]
fn blargg_apu_01_len_ctr() {
    run_test("apu_01_len_ctr", "tests/roms/apu_01_len_ctr.nes", 600);
}

#[test]
fn blargg_apu_02_len_table() {
    run_test("apu_02_len_table", "tests/roms/apu_02_len_table.nes", 600);
}

#[test]
fn blargg_apu_03_irq_flag() {
    run_test("apu_03_irq_flag", "tests/roms/apu_03_irq_flag.nes", 600);
}

#[test]
fn blargg_apu_05_len_timing_mode0() {
    run_test(
        "apu_05_len_timing_mode0",
        "tests/roms/apu_05_len_timing_mode0.nes",
        600,
    );
}

#[test]
fn blargg_apu_06_len_timing_mode1() {
    run_test(
        "apu_06_len_timing_mode1",
        "tests/roms/apu_06_len_timing_mode1.nes",
        600,
    );
}

#[test]
fn blargg_apu_07_irq_flag_timing() {
    run_test(
        "apu_07_irq_flag_timing",
        "tests/roms/apu_07_irq_flag_timing.nes",
        600,
    );
}

#[test]
fn blargg_apu_08_irq_timing() {
    run_test("apu_08_irq_timing", "tests/roms/apu_08_irq_timing.nes", 600);
}

#[test]
fn blargg_apu_04_clock_jitter() {
    run_test(
        "apu_04_clock_jitter",
        "tests/roms/apu_04_clock_jitter.nes",
        600,
    );
}

#[test]
fn blargg_apu_09_reset_timing() {
    run_test(
        "apu_09_reset_timing",
        "tests/roms/apu_09_reset_timing.nes",
        600,
    );
}

#[test]
fn blargg_apu_10_len_halt_timing() {
    run_test(
        "apu_10_len_halt_timing",
        "tests/roms/apu_10_len_halt_timing.nes",
        600,
    );
}

#[test]
fn blargg_apu_11_len_reload_timing() {
    run_test(
        "apu_11_len_reload_timing",
        "tests/roms/apu_11_len_reload_timing.nes",
        600,
    );
}
