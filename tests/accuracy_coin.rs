use std::fs;
use std::path::Path;

use nes::bus::Bus;
use nes::cpu::CpuRp2a03;
use nes::rom::Rom;
use nes::{reset, tick};

const RESULT_START: u16 = 0x0400;
const RESULT_END: u16 = 0x0492;

/// Run the AccuracyCoin ROM headless, optionally pressing Start to run all tests.
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

    fn press_start(&mut self) {
        self.bus.pad1.start = true;
    }

    fn release_start(&mut self) {
        self.bus.pad1.start = false;
    }

    fn read_ram(&self, addr: u16) -> u8 {
        self.bus.ram[(addr & 0x07FF) as usize]
    }

    fn dump_results(&self) -> Vec<(u16, u8)> {
        let mut out = Vec::new();
        for addr in RESULT_START..=RESULT_END {
            let val = self.read_ram(addr);
            if val != 0 {
                out.push((addr, val));
            }
        }
        out
    }
}

/// Result status interpretation
fn describe_result(val: u8) -> &'static str {
    match val & 0x03 {
        1 => "PASS",
        2 => "FAIL",
        _ => "SKIP",
    }
}

fn error_number(val: u8) -> u8 {
    val >> 2
}

/// Map result address to test name
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

#[test]
fn accuracy_coin_memory_map() {
    let rom_path = Path::new("tests/accuracy_coin.nes");
    assert!(rom_path.exists());

    let data = fs::read(rom_path).unwrap();
    let rom = Rom::new(&data).expect("valid ROM");

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

    // Run step by step with debug output
    let mut runner = AccuracyCoinRunner::new(rom_path);

    for frame_check in &[3, 5, 10, 20, 30, 50, 100, 200] {
        runner.run_frames(*frame_check - runner.total_frames);
        let debug = runner.read_ram(0xEC);
        let pc = runner.cpu.pc();
        println!(
            "Frame {}: Debug $EC=${:02X}, PC=${:04X}",
            runner.total_frames, debug, pc
        );
        if debug == 0x0A {
            break;
        }
    }

    let debug = runner.read_ram(0xEC);
    let magic = runner.read_ram(0x3F0);
    let pc = runner.cpu.pc();
    println!(
        "Final: Debug $EC=${:02X}, Magic $3F0=${:02X}, PC=${:04X}",
        debug, magic, pc
    );

    assert_eq!(
        debug, 0x0A,
        "ROM didn't reach main menu after {} frames (Debug $EC = ${:02X})",
        runner.total_frames, debug
    );
    assert_eq!(magic, 0x5A, "Power-on magic not set");
}

#[test]
fn accuracy_coin_run_all_tests() {
    let rom_path = Path::new("tests/accuracy_coin.nes");
    assert!(rom_path.exists());

    let mut runner = AccuracyCoinRunner::new(rom_path);

    // Boot to main menu (need extra frames for DMASync timeout path)
    runner.run_frames(200);

    let debug = runner.read_ram(0xEC);
    println!("Debug $EC after 200 frames: ${:02X}", debug);
    // The ROM reaches $0A once it passes DMASync and loads the menu
    // If it's stuck at $06, DMASync is taking longer (DMC DMA path)
    if debug != 0x0A {
        runner.run_frames(800);
        let debug = runner.read_ram(0xEC);
        println!("Debug $EC after 1000 frames total: ${:02X}", debug);
        assert_eq!(
            debug, 0x0A,
            "ROM didn't boot to main menu after 1000 frames"
        );
    }

    // Press Start to run all tests
    // The NMI reads the controller each frame, so we need to hold Start
    runner.press_start();

    // Run enough frames for all tests to complete
    // AccuracyCoin has ~170+ tests, each takes 1-5 frames
    // plus overhead from menu transitions
    runner.run_frames(6000); // ~100 seconds

    runner.release_start();

    let pc = runner.cpu.pc();
    let frames = runner.total_frames;
    let cycles = runner.total_cycles;
    println!(
        "After test run: PC=${:04X}, {} frames, {} cycles",
        pc, frames, cycles
    );

    // Read detailed RAM for debugging
    println!("\n=== Debug RAM Dump ===");
    for (addr, name) in &[
        (0xCA, "$CA"),
        (0x2EA, "$02EA"),
        (0x56, "$56"),
        (0x08, "open bus"),
        (0x10, "ErrorCode"),
        (0x50, "$50"),
        (0x51, "$51"),
        (0x52, "$52"),
        (0xFA, "Copy_SP"),
        (0xFB, "Copy_SP2"),
        (0xFC, "Copy_Flags"),
        (0xFD, "Copy_X"),
        (0xFE, "Copy_Y"),
        (0xFF, "Copy_A"),
    ] {
        println!("  {} = ${:02X}", name, runner.read_ram(*addr));
    }

    // Read results
    let results = runner.dump_results();
    println!("\n=== AccuracyCoin Test Results ===");
    println!("Total non-zero results: {}", results.len());

    let mut pass_count = 0;
    let mut fail_count = 0;
    let mut skip_count = 0;
    let mut other_count = 0;

    for (addr, val) in &results {
        let status = describe_result(*val);
        match *val & 0x03 {
            1 => pass_count += 1,
            2 => fail_count += 1,
            3 => skip_count += 1,
            _ => other_count += 1,
        }
        if *val & 0x03 != 1 || *val > 0x03 {
            let name = test_name(*addr);
            let err = error_number(*val);
            println!(
                "  ${:04X} [{}]: {} (value=${:02X}, error={})",
                addr, name, status, val, err
            );
        }
    }

    println!("\nSummary:");
    println!("  PASS: {}", pass_count);
    println!("  FAIL: {}", fail_count);
    println!("  SKIP: {}", skip_count);
    println!("  OTHER: {}", other_count);
    println!("  TOTAL: {}", results.len());
}

/// Test known-working CPU and addressing features individually
/// without relying on the AccuracyCoin ROM's full automation
#[test]
fn accuracy_coin_run_cpu_suite() {
    let rom_path = Path::new("tests/accuracy_coin.nes");
    assert!(rom_path.exists());

    let mut runner = AccuracyCoinRunner::new(rom_path);

    // Boot to main menu (may take up to 50 frames for DMASync)
    runner.run_frames(200);
    assert_eq!(
        runner.read_ram(0xEC),
        0x0A,
        "ROM didn't boot after 200 frames"
    );

    // Navigate to the first test and run it
    // Press Down twice to move cursor from top to first test
    runner.bus.pad1.down = true;
    runner.run_frames(3);
    runner.bus.pad1.down = false;
    runner.run_frames(2);

    // Press A to run the first test
    runner.bus.pad1.a = true;
    runner.run_frames(1);
    runner.bus.pad1.a = false;
    runner.run_frames(15);

    // Read the CPU Behavior test results
    println!("\nCPU Behavior page results (after running first test):");
    for addr in 0x0400..=0x0408 {
        let val = runner.read_ram(addr);
        if val != 0 {
            let status = describe_result(val);
            let name = test_name(addr);
            println!("  ${:04X} [{}]: {} (val=${:02X})", addr, name, status, val);
        }
    }
}

/// Test the ROM boot process and power-on state checks
#[test]
fn accuracy_coin_boot_state() {
    let rom_path = Path::new("tests/accuracy_coin.nes");
    assert!(rom_path.exists());

    let mut runner = AccuracyCoinRunner::new(rom_path);
    runner.run_frames(30);

    // Check power-on state captures
    let power_a = runner.read_ram(0x370); // PowerOn_A
    let power_x = runner.read_ram(0x371); // PowerOn_X
    let power_y = runner.read_ram(0x372); // PowerOn_Y
    let power_sp = runner.read_ram(0x373); // PowerOn_SP
    let power_p = runner.read_ram(0x374); // PowerOn_P
    let ppu_reset = runner.read_ram(0x360); // PowerOnTest_PPUReset

    println!("Power-On State:");
    println!("  A={:02X}, X={:02X}, Y={:02X}", power_a, power_x, power_y);
    println!("  SP={:02X}, P={:02X}", power_sp, power_p);
    println!("  PPU Reset={:02X}", ppu_reset);

    // A should be non-zero after reset (it typically reads the reset vector's first byte)
    // SP should be $FD after power-on
    println!("  Power-On Magic at $3F0: ${:02X}", runner.read_ram(0x3F0));

    // Check PPU Reset flag test
    // Value of 1 = pass (PPU has reset flag), 6 = fail (no reset flag)
    // On a real NES/RP2C02G, PPU reset flag exists
    println!(
        "  PPU Reset Flag test: value=${:02X} (1=pass, 6=fail)",
        ppu_reset
    );
}
