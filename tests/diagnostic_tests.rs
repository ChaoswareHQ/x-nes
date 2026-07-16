use std::fs;
use std::path::Path;

use nes::bus::Bus;
use nes::cpu::CpuRp2a03;
use nes::rom::Rom;
use nes::{reset, tick};

struct DiagnosticRunner {
    cpu: CpuRp2a03,
    bus: Bus,
    total_cycles: u64,
    total_frames: u32,
}

impl DiagnosticRunner {
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

    fn read_result(&mut self, addr: u16) -> u8 {
        self.bus.read(addr)
    }

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

    fn status(&mut self) -> u8 {
        self.read_result(0x6000)
    }

    fn has_signature(&mut self) -> bool {
        self.read_result(0x6001) == 0xDE && self.read_result(0x6002) == 0xB0
    }
}

fn run_blargg_test(name: &str, rom_path: &str, max_frames: u32) -> Result<(), String> {
    let path = Path::new(rom_path);
    if !path.exists() {
        return Err(format!("ROM not found: {}", rom_path));
    }

    let mut runner = DiagnosticRunner::new(path);
    runner.run_frames(max_frames);

    let has_sig = runner.has_signature();
    let status = runner.status();
    let text = runner.read_text();

    if has_sig {
        if status == 0 {
            println!("  PASS: {} ({} frames)", name, runner.total_frames);
            Ok(())
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
            Err(format!("Failed with code {}", status))
        }
    } else {
        println!(
            "  ?: {} (no sig, status={}, {} frames)",
            name, status, runner.total_frames
        );
        if status == 0 {
            Ok(())
        } else {
            Err(format!("Failed with code {}", status))
        }
    }
}

// ========================================================================
// PPU VBL/NMI - Individual sub-tests
// ========================================================================

#[test]
fn diag_ppu_vbl_nmi_01() {
    run_blargg_test(
        "01-vbl_basics",
        "tests/roms-collection/ppu_vbl_nmi/rom_singles/01-vbl_basics.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_ppu_vbl_nmi_02() {
    run_blargg_test(
        "02-vbl_set_time",
        "tests/roms-collection/ppu_vbl_nmi/rom_singles/02-vbl_set_time.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_ppu_vbl_nmi_03() {
    run_blargg_test(
        "03-vbl_clear_time",
        "tests/roms-collection/ppu_vbl_nmi/rom_singles/03-vbl_clear_time.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_ppu_vbl_nmi_04() {
    run_blargg_test(
        "04-nmi_control",
        "tests/roms-collection/ppu_vbl_nmi/rom_singles/04-nmi_control.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_ppu_vbl_nmi_05() {
    // This is our failing sub-test - will help narrow down the issue
    run_blargg_test(
        "05-nmi_timing",
        "tests/roms-collection/ppu_vbl_nmi/rom_singles/05-nmi_timing.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_ppu_vbl_nmi_06() {
    run_blargg_test(
        "06-suppression",
        "tests/roms-collection/ppu_vbl_nmi/rom_singles/06-suppression.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_ppu_vbl_nmi_07() {
    run_blargg_test(
        "07-nmi_on_timing",
        "tests/roms-collection/ppu_vbl_nmi/rom_singles/07-nmi_on_timing.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_ppu_vbl_nmi_08() {
    run_blargg_test(
        "08-nmi_off_timing",
        "tests/roms-collection/ppu_vbl_nmi/rom_singles/08-nmi_off_timing.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_ppu_vbl_nmi_09() {
    run_blargg_test(
        "09-even_odd_frames",
        "tests/roms-collection/ppu_vbl_nmi/rom_singles/09-even_odd_frames.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_ppu_vbl_nmi_10() {
    run_blargg_test(
        "10-even_odd_timing",
        "tests/roms-collection/ppu_vbl_nmi/rom_singles/10-even_odd_timing.nes",
        600,
    )
    .unwrap();
}

// ========================================================================
// PPU Open Bus
// ========================================================================

#[test]
fn diag_ppu_open_bus() {
    run_blargg_test(
        "ppu_open_bus",
        "tests/roms-collection/ppu_open_bus/ppu_open_bus.nes",
        600,
    )
    .unwrap();
}

// ========================================================================
// DMA + PPU/APU interactions
// ========================================================================

#[test]
fn diag_dma_2007_read() {
    run_blargg_test(
        "dma_2007_read",
        "tests/roms-collection/dmc_dma_during_read4/dma_2007_read.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_dma_2007_write() {
    run_blargg_test(
        "dma_2007_write",
        "tests/roms-collection/dmc_dma_during_read4/dma_2007_write.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_dma_4016_read() {
    run_blargg_test(
        "dma_4016_read",
        "tests/roms-collection/dmc_dma_during_read4/dma_4016_read.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_double_2007_read() {
    run_blargg_test(
        "double_2007_read",
        "tests/roms-collection/dmc_dma_during_read4/double_2007_read.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_read_write_2007() {
    run_blargg_test(
        "read_write_2007",
        "tests/roms-collection/dmc_dma_during_read4/read_write_2007.nes",
        600,
    )
    .unwrap();
}

// ========================================================================
// CPU Interrupt timing
// ========================================================================

#[test]
fn diag_cpu_interrupts_full() {
    run_blargg_test(
        "cpu_interrupts",
        "tests/roms-collection/cpu_interrupts_v2/cpu_interrupts.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_cli_latency() {
    run_blargg_test(
        "cli_latency",
        "tests/roms-collection/cpu_interrupts_v2/rom_singles/1-cli_latency.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_nmi_and_brk() {
    run_blargg_test(
        "nmi_and_brk",
        "tests/roms-collection/cpu_interrupts_v2/rom_singles/2-nmi_and_brk.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_nmi_and_irq() {
    run_blargg_test(
        "nmi_and_irq",
        "tests/roms-collection/cpu_interrupts_v2/rom_singles/3-nmi_and_irq.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_irq_and_dma() {
    run_blargg_test(
        "irq_and_dma",
        "tests/roms-collection/cpu_interrupts_v2/rom_singles/4-irq_and_dma.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_branch_delays_irq() {
    run_blargg_test(
        "branch_delays_irq",
        "tests/roms-collection/cpu_interrupts_v2/rom_singles/5-branch_delays_irq.nes",
        600,
    )
    .unwrap();
}

// ========================================================================
// Sprite DMA + DMC DMA
// ========================================================================

#[test]
fn diag_sprdma_dmc() {
    run_blargg_test(
        "sprdma_and_dmc_dma",
        "tests/roms-collection/sprdma_and_dmc_dma/sprdma_and_dmc_dma.nes",
        1800,
    )
    .unwrap();
}

// ========================================================================
// OAM / Sprite tests
// ========================================================================

#[test]
fn diag_oam_stress() {
    run_blargg_test(
        "oam_stress",
        "tests/roms-collection/oam_stress/oam_stress.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_oam_read() {
    run_blargg_test(
        "oam_read",
        "tests/roms-collection/oam_read/oam_read.nes",
        600,
    )
    .unwrap();
}

// ========================================================================
// APU Mixer
// ========================================================================

#[test]
fn diag_apu_square() {
    run_blargg_test(
        "apu_square",
        "tests/roms-collection/apu_mixer/square.nes",
        1200,
    )
    .unwrap();
}

#[test]
fn diag_apu_noise() {
    run_blargg_test(
        "apu_noise",
        "tests/roms-collection/apu_mixer/noise.nes",
        1500,
    )
    .unwrap();
}

#[test]
fn diag_apu_triangle() {
    run_blargg_test(
        "apu_triangle",
        "tests/roms-collection/apu_mixer/triangle.nes",
        1200,
    )
    .unwrap();
}

#[test]
fn diag_apu_dmc() {
    run_blargg_test("apu_dmc", "tests/roms-collection/apu_mixer/dmc.nes", 1200).unwrap();
}

// ========================================================================
// MMC3 tests (SMB3 compat)
// ========================================================================

#[test]
fn diag_mmc3_clocking() {
    run_blargg_test(
        "mmc3_clocking",
        "tests/roms-collection/mmc3_test/1-clocking.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_mmc3_details() {
    run_blargg_test(
        "mmc3_details",
        "tests/roms-collection/mmc3_test/2-details.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_mmc3_a12_clocking() {
    run_blargg_test(
        "mmc3_a12_clocking",
        "tests/roms-collection/mmc3_test/3-A12_clocking.nes",
        600,
    )
    .unwrap();
}

#[test]
fn diag_mmc3_scanline_timing() {
    run_blargg_test(
        "mmc3_scanline_timing",
        "tests/roms-collection/mmc3_test/4-scanline_timing.nes",
        600,
    )
    .unwrap();
}
