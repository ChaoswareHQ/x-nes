use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

use nes::bus::Bus;
use nes::cpu::CpuRp2a03;
use nes::rom::Rom;
use nes::{reset, tick};

const RESULT_START: u16 = 0x0400;
const RESULT_END: u16 = 0x0492;

/// Result status encoding from AccuracyCoin ROM
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestStatus {
    Skip,
    Pass,
    Fail(u8), // error number
}

impl TestStatus {
    fn from_val(val: u8) -> Self {
        match val & 0x03 {
            1 => TestStatus::Pass,
            2 => TestStatus::Fail(val >> 2),
            _ => TestStatus::Skip,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            TestStatus::Skip => "SKIP",
            TestStatus::Pass => "PASS",
            TestStatus::Fail(_) => "FAIL",
        }
    }
}

/// A single AccuracyCoin test result.
#[derive(Debug)]
struct TestResult {
    addr: u16,
    name: &'static str,
    status: TestStatus,
    raw: u8,
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

struct AccuracyCoinRunner {
    cpu: CpuRp2a03,
    bus: Bus,
    total_cycles: u64,
    total_frames: u32,
}

impl AccuracyCoinRunner {
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

    fn tick(&mut self) {
        let cycles = tick(&mut self.cpu, &mut self.bus);
        self.total_cycles += cycles as u64;
        if self.bus.ppu.frame_complete {
            self.bus.ppu.frame_complete = false;
            self.total_frames += 1;
        }
    }

    fn run_frames(&mut self, n: u32) {
        let target = self.total_frames + n;
        let max_cycles = 200_000_000u64;
        loop {
            self.tick();
            if self.total_frames >= target {
                break;
            }
            if self.total_cycles > max_cycles {
                break;
            }
        }
    }

    fn read_ram(&self, addr: u16) -> u8 {
        self.bus.ram[(addr & 0x07FF) as usize]
    }

    fn press_start(&mut self) {
        self.bus.pad1.start = true;
    }

    fn release_start(&mut self) {
        self.bus.pad1.start = false;
    }

    fn collect_results(&self) -> Vec<TestResult> {
        let mut out = Vec::new();
        for addr in RESULT_START..=RESULT_END {
            let val = self.read_ram(addr);
            if val != 0 {
                out.push(TestResult {
                    addr,
                    name: test_name(addr),
                    status: TestStatus::from_val(val),
                    raw: val,
                });
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Test name map
// ---------------------------------------------------------------------------

fn test_name(addr: u16) -> &'static str {
    match addr {
        0x0400 => "Unimplemented",
        0x0401 => "CPU Instructions",
        0x0403 => "RAM Mirroring",
        0x0404 => "PPU Register Mirroring",
        0x0405 => "ROM Not Writable",
        0x0406 => "Dummy Reads",
        0x0407 => "Dummy Writes",
        0x0408 => "Open Bus",
        0x0409..=0x040F => "SLO",
        0x0410 => "ANC $0B",
        0x0411 => "ANC $2B",
        0x0412 => "ASR $4B",
        0x0413 => "ARR $6B",
        0x0414 => "ANE $8B",
        0x0415 => "LXA $AB",
        0x0416 => "AXS $CB",
        0x0417 => "SBC $EB",
        0x0419..=0x041F => "RLA",
        0x0420 => "SRE $43",
        0x0422..=0x0426 => "SRE",
        0x0427..=0x042D => "RRA",
        0x042E..=0x0431 => "SAX",
        0x0432..=0x0437 => "LAX",
        0x0438..=0x043E => "DCP",
        0x043F..=0x0445 => "ISC",
        0x0446 => "SHA $93",
        0x0447 => "SHA $9F",
        0x0448 => "SHS $9B",
        0x0449 => "SHY $9C",
        0x044A => "SHX $9E",
        0x044B => "LAE $BB",
        0x044C => "DMA + $2007 Read",
        0x044D => "PC Wraparound",
        0x044E => "PPU Open Bus",
        0x044F => "DMA + $2007 Write",
        0x0450 => "VBlank Beginning",
        0x0451 => "VBlank End",
        0x0452 => "NMI Control",
        0x0453 => "NMI Timing",
        0x0454 => "NMI Suppression",
        0x0455 => "NMI VBlank End",
        0x0456 => "NMI Disabled VBlank",
        0x0457 => "Sprite 0 Hit",
        0x0458 => "Arbitrary Sprite Zero",
        0x0459 => "Sprite Overflow",
        0x045A => "Misaligned OAM",
        0x045B => "$2004 Behavior",
        0x045C => "APU Register Activation",
        0x045D => "DMA + $4015 Read",
        0x045E => "DMA + $4016 Read",
        0x045F => "Controller Strobing",
        0x0460 => "Instruction Timing",
        0x0461 => "I-Flag Latency",
        0x0462 => "NMI + BRK",
        0x0463 => "NMI + IRQ",
        0x0464 => "RMW $2007",
        0x0465 => "APU Length Counter",
        0x0466 => "APU Length Table",
        0x0467 => "Frame Counter IRQ",
        0x0468 => "Frame Counter 4-Step",
        0x0469 => "Frame Counter 5-Step",
        0x046A => "Delta Modulation Channel",
        0x046B => "DMA Bus Conflict",
        0x046C => "DMA + Open Bus",
        0x046D => "Implied Dummy Reads",
        0x046E => "Addr Mode Abs/Index",
        0x046F => "Addr Mode ZP Indexed",
        0x0470 => "Addr Mode Indirect",
        0x0471 => "Addr Mode (Indirect,X)",
        0x0472 => "Addr Mode (Indirect),Y",
        0x0473 => "Addr Mode Relative",
        0x0474 => "Decimal Flag",
        0x0475 => "B Flag",
        0x0476 => "PPU Read Buffer",
        0x0477 => "DMC DMA + OAM DMA",
        0x0478 => "Implicit DMA Abort",
        0x0479 => "Explicit DMA Abort",
        0x047A => "Controller Clocking",
        0x047B => "OAM Corruption",
        0x047C => "JSR Edge Cases",
        0x047D => "All NOPs",
        0x047E => "Palette RAM Quirks",
        0x0480 => "INC $4014",
        0x0481 => "Attributes As Tiles",
        0x0482 => "t Register Quirks",
        0x0483 => "Stale BG Shift Regs",
        0x0484 => "Scanline 0 Sprites",
        0x0485 => "CHR ROM Not Writable",
        0x0486 => "Rendering Flag Behavior",
        0x0487 => "BG Serial In",
        0x0488 => "DMA + $2002 Read",
        0x0489 => "Suddenly Resize Sprite",
        0x048A => "$2007 w/ Rendering",
        0x048B => "Branch Dummy Reads",
        0x048C => "$2004 Stress",
        0x048D => "$2002 Flag Timing",
        0x048E => "$2007 Stress",
        0x048F => "Stale Sprite Shift Regs",
        0x0490 => "Internal Data Bus",
        0x0491 => "ALE + Read",
        0x0492 => "Hybrid Addresses",
        _ => "Unknown",
    }
}

fn test_subsystem(addr: u16) -> &'static str {
    match addr {
        0x0400..=0x0408 => "CPU-Basics",
        0x0409..=0x0417 => "CPU-Illegal",
        0x0419..=0x044B => "CPU-Illegal",
        0x044C..=0x044F => "PPU-DMA",
        0x0450..=0x045B => "PPU",
        0x045C..=0x046A => "APU",
        0x046B..=0x046C => "DMA",
        0x046D..=0x0475 => "CPU",
        0x0476 => "PPU",
        0x0477..=0x0479 => "DMA",
        0x047A => "Controller",
        0x047B => "PPU-OAM",
        0x047C => "CPU",
        0x047D => "CPU-NOPs",
        0x047E..=0x048F => "PPU",
        0x0490 => "CPU",
        0x0491 => "CPU",
        0x0492 => "CPU",
        _ => "Unknown",
    }
}

// ---------------------------------------------------------------------------
// Printing helpers
// ---------------------------------------------------------------------------

fn print_result_table(results: &[TestResult]) {
    if results.is_empty() {
        println!("  (no results recorded)");
        return;
    }

    // Group by subsystem
    let mut groups: BTreeMap<&str, Vec<&TestResult>> = BTreeMap::new();
    for r in results {
        groups.entry(test_subsystem(r.addr)).or_default().push(r);
    }

    for (sys, sys_results) in &groups {
        let pass = sys_results.iter().filter(|r| r.status == TestStatus::Pass).count();
        let fail = sys_results.iter().filter(|r| matches!(r.status, TestStatus::Fail(_))).count();
        let skip = sys_results.iter().filter(|r| r.status == TestStatus::Skip).count();

        println!("  ┌─ {sys} ─────────────────────────────────────");
        println!("  │  {pass} passed, {fail} failed, {skip} skipped");

        for r in sys_results {
            let marker = match r.status {
                TestStatus::Pass => "✓",
                TestStatus::Fail(_) => "✗",
                TestStatus::Skip => "‒",
            };
            match &r.status {
                TestStatus::Fail(err) => {
                    println!("  │   {marker} ${:04X} {:<32} FAIL  err={err}", r.addr, r.name);
                }
                TestStatus::Skip => {
                    println!("  │   {marker} ${:04X} {:<32} SKIP", r.addr, r.name);
                }
                _ => {}
            }
        }
    }
}

fn print_summary(results: &[TestResult], elapsed: std::time::Duration, frames: u32, cycles: u64) {
    let pass = results.iter().filter(|r| r.status == TestStatus::Pass).count();
    let fail = results.iter().filter(|r| matches!(r.status, TestStatus::Fail(_))).count();
    let skip = results.iter().filter(|r| r.status == TestStatus::Skip).count();

    println!();
    println!("══════════════════════════════════════════════════");
    println!("  AccuracyCoin Test Summary");
    println!("══════════════════════════════════════════════════");
    println!("  Duration:  {elapsed:.2?}");
    println!("  Frames:    {frames}");
    println!("  Cycles:    {cycles}");
    println!("  µs/frame:  {:.1}", elapsed.as_secs_f64() * 1_000_000.0 / frames as f64);
    println!("  cycles/f:  {:.0}", cycles as f64 / frames as f64);
    println!();
    println!("  {pass:>3}  PASS");
    println!("  {fail:>3}  FAIL");
    println!("  {skip:>3}  SKIP");
    println!("  ─────");
    println!("  {:>3}  TOTAL", results.len());
    println!("══════════════════════════════════════════════════");
}

// ---------------------------------------------------------------------------
// Boot-time debug state labels
// ---------------------------------------------------------------------------

fn debug_state_label(val: u8) -> &'static str {
    match val {
        0x00 => "POWERON_INIT",
        0x01 => "CHR_CHECK",
        0x02 => "DMASync",
        0x06 => "DMASyncTimeout",
        0x07 => "VSync",
        0x08 => "ScreensInit",
        0x09 => "LoadMenu",
        0x0A => "ShowMenu",
        0x0B => "WaitStart",
        0x0C => "RunTests",
        0x0D => "NextTest",
        0x0E => "PrevTest",
        0x0F => "WaitStartTest",
        0x10 => "TestCheck",
        0x11 => "Waiting",
        0x12 => "Compare",
        0x13 => "Exit",
        0xFF => "Error",
        _ => "???",
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[test]
fn accuracy_coin_memory_map() {
    let rom_path = Path::new("tests/accuracy_coin.nes");
    assert!(rom_path.exists(), "ROM file not found — download accuracy_coin.nes");

    let data = fs::read(rom_path).unwrap();
    let rom = Rom::new(&data).expect("valid iNES ROM");

    assert_eq!(rom.mapper_id, 0, "AccuracyCoin should be mapper 0 (NROM)");
    assert_eq!(rom.prg.len(), 0x8000, "Expected 32KB PRG ROM");
    assert_eq!(rom.chr.len(), 0x2000, "Expected 8KB CHR ROM");

    let name = b"CPU Behavior";
    assert!(
        rom.prg.windows(name.len()).any(|w| w == name),
        "Should find 'CPU Behavior' string in ROM"
    );
}

#[test]
fn accuracy_coin_boots_to_menu() {
    let rom_path = Path::new("tests/accuracy_coin.nes");
    assert!(rom_path.exists());

    let start = Instant::now();
    let mut runner = AccuracyCoinRunner::new(rom_path);

    // Poll debug state until we reach the menu or hit our frame limit
    let max_frames = 200;
    runner.run_frames(max_frames);

    let debug = runner.read_ram(0xEC);
    let magic = runner.read_ram(0x3F0);

    println!("Boot path (Debug $EC): ${:02X} — {}", debug, debug_state_label(debug));
    println!("Magic $3F0: ${:02X}", magic);
    println!("Frames to boot: {} / {max_frames}", runner.total_frames);
    println!("Elapsed: {:.2?}", start.elapsed());

    assert_eq!(
        debug, 0x0A,
        "ROM didn't reach main menu (got ${:02X} = {})",
        debug, debug_state_label(debug)
    );
    assert_eq!(magic, 0x5A, "Power-on magic signature not set");
}

/// Boot to menu, press Start, and run every AccuracyCoin test.
/// Asserts that any test designated PASS actually passed, and FAIL results
/// are reported with their error codes for debugging.
#[test]
fn accuracy_coin_run_all_tests() {
    let rom_path = Path::new("tests/accuracy_coin.nes");
    assert!(rom_path.exists());

    let start = Instant::now();
    let mut runner = AccuracyCoinRunner::new(rom_path);

    // -- Phase 1: Boot to menu ------------------------------------------------
    runner.run_frames(1000);
    let debug = runner.read_ram(0xEC);
    assert_eq!(
        debug, 0x0A,
        "ROM didn't reach main menu after 1000 frames (Debug $EC = ${:02X} = {})",
        debug, debug_state_label(debug)
    );
    let boot_elapsed = start.elapsed();
    println!("[Phase 1] Booted to menu — {} frames, {boot_elapsed:.2?}", runner.total_frames);

    // -- Phase 2: Run all tests -----------------------------------------------
    runner.press_start();
    runner.run_frames(6000);
    runner.release_start();
    let run_elapsed = start.elapsed() - boot_elapsed;
    println!("[Phase 2] Test run complete — {} additional frames, {run_elapsed:.2?}", 6000);

    // -- Phase 3: Collect and report results ----------------------------------
    let results = runner.collect_results();
    let total_elapsed = start.elapsed();
    let total = results.len();

    println!();
    println!("── Results by subsystem ──");
    print_result_table(&results);
    print_summary(&results, total_elapsed, runner.total_frames, runner.total_cycles);

    // -- Assert ---------------------------------------------------------------
    // If NO results at all, something went wrong.
    assert!(total > 0, "No test results recorded — ROM may not have run the tests");

    // Collect failures.
    let fails: Vec<&TestResult> = results.iter().filter(|r| matches!(r.status, TestStatus::Fail(_))).collect();

    // Skip-check: if tests are skipped because they don't apply (e.g. PAL-only features on NTSC),
    // that's fine. But if expected tests are silent (val == 0), we only warn.
    let skip_critical = results.iter().any(|r| {
        r.status == TestStatus::Skip && matches!(r.addr, 0x0401 /* CPU Instructions */)
    });

    if skip_critical {
        println!("WARNING: CPU Instructions test was SKIPPED (value may still be zero at read time)");
    }

    if !fails.is_empty() {
        println!();
        println!("FAILURES DETECTED — see above for details");
        // Print a concise failure line for the assertion message.
        for r in &fails {
            let err = match r.status {
                TestStatus::Fail(n) => n,
                _ => 0,
            };
            println!("  FAIL  ${:04X}  {:<32}  error={err}", r.addr, r.name);
        }
        panic!("{} test(s) failed", fails.len());
    }
}

/// Boot, navigate to the CPU instruction test via menu, and check its results.
#[test]
fn accuracy_coin_run_cpu_suite() {
    let rom_path = Path::new("tests/accuracy_coin.nes");
    assert!(rom_path.exists());

    let start = Instant::now();
    let mut runner = AccuracyCoinRunner::new(rom_path);

    // Boot to menu
    runner.run_frames(200);
    assert_eq!(
        runner.read_ram(0xEC),
        0x0A,
        "ROM didn't boot after 200 frames"
    );

    // Navigate to first test: the menu cursor starts at index 0 (CPU Instructions).
    // Press A to select it.
    runner.bus.pad1.a = true;
    runner.run_frames(1);
    runner.bus.pad1.a = false;

    // Wait for the test to run and populate results
    runner.run_frames(120);

    println!();
    println!("── CPU Instruction Tests ──");
    println!("Time: {:.2?}", start.elapsed());

    let mut pass = 0;
    let mut fail = 0;
    for addr in 0x0400..=0x0408 {
        let val = runner.read_ram(addr);
        if val == 0 {
            continue;
        }
        let status = TestStatus::from_val(val);
        let name = test_name(addr);
        let marker = match status {
            TestStatus::Pass => { pass += 1; "✓" }
            TestStatus::Fail(_) => { fail += 1; "✗" }
            TestStatus::Skip => "‒"
        };
        if let TestStatus::Fail(err) = status {
            println!("  {marker} ${addr:04X} {name:<30} FAIL  err={err}  (val=${val:02X})");
        }
    }

    assert!(
        pass > 0 || fail > 0,
        "No CPU baseline test results at all — menu navigation may have failed"
    );
    assert_eq!(
        fail, 0,
        "{fail} CPU baseline test(s) failed (see above)"
    );
    println!("  ✓ {pass} passed, {fail} failed");
}

/// Check power-on register state as captured by AccuracyCoin.
#[test]
fn accuracy_coin_boot_state() {
    let rom_path = Path::new("tests/accuracy_coin.nes");
    assert!(rom_path.exists());

    let start = Instant::now();
    let mut runner = AccuracyCoinRunner::new(rom_path);
    runner.run_frames(30);

    let power_a  = runner.read_ram(0x370);
    let power_x  = runner.read_ram(0x371);
    let power_y  = runner.read_ram(0x372);
    let power_sp = runner.read_ram(0x373);
    let power_p_reg = runner.read_ram(0x374);
    let ppu_rs   = runner.read_ram(0x360);

    println!("Power-On State  (time: {:.2?})", start.elapsed());
    println!("  A  = ${power_a:02X}");
    println!("  X  = ${power_x:02X}");
    println!("  Y  = ${power_y:02X}");
    println!("  SP = ${power_sp:02X}   (expected $FD)");
    println!("  P  = ${power_p_reg:02X}   (expected bit 5 = 1)");
    println!("  PPU Reset Flag = ${ppu_rs:02X}   (1 = pass, 6 = fail)");

    assert_eq!(
        power_sp, 0xFD,
        "SP should be $FD after reset, got ${power_sp:02X}"
    );
    assert!(
        power_p_reg & 0x20 != 0,
        "P should have bit 5 set after reset, got ${power_p_reg:02X}"
    );
    assert!(
        power_p_reg & 0x04 != 0,
        "I flag (bit 2) should be set after reset, got ${power_p_reg:02X}"
    );
    assert_eq!(
        ppu_rs, 1,
        "PPU Reset Flag test failed — value ${ppu_rs:02X} (expected 1)"
    );
    println!("  ✓ All power-on assertions passed");
}
