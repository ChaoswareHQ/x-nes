/// Debugging example for x-nes.
///
/// Run with various flags to inspect emulator state:
///
///   # Basic run (10 frames, no debug output):
///   cargo run --example debug -- "rom.nes"
///
///   # Run 100 frames, dump PPU state at frame 5:
///   cargo run --example debug -- "rom.nes" --frames 100 --dump-ppu 5
///
///   # Dump palette + save frame as PPM:
///   cargo run --example debug -- "rom.nes" --dump-palette 10 --dump-frame 10
///
///   # Everything at frame 20:
///   cargo run --example debug -- "rom.nes" --frames 60 --dump-all 20
///
///   # Quick diagnostic at frame 30, save frame 30 as PPM:
///   cargo run --example debug -- "rom.nes" --frames 60 --diag 30 --dump-frame 30
///
///   # Trace MMC5 register writes (verbose):
///   cargo run --example debug -- "rom.nes" --trace-mapper
///
///   # Live PC tracing:
///   cargo run --example debug -- "rom.nes" --trace-pc
use nes::bus::Bus;
use nes::cpu::CpuRp2a03;
use nes::debug;
use nes::rom::Rom;
use nes::{reset, tick};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <rom.nes> [options]", args[0]);
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --frames N         Run for N frames (default: 60)");
        eprintln!("  --dump-frame N     Save frame N as frame_N.ppm");
        eprintln!("  --dump-ppu N       Dump PPU state at frame N");
        eprintln!("  --dump-palette N   Dump palette at frame N");
        eprintln!("  --dump-oam N       Dump sprite memory at frame N");
        eprintln!("  --dump-nt N        Dump nametables at frame N");
        eprintln!("  --dump-chr N       Dump CHR page at frame N");
        eprintln!("  --diag N           Quick diagnostics at frame N");
        eprintln!("  --dump-mmc5 N      Dump MMC5 registers + ExRAM at frame N");
        eprintln!("  --dump-all N       All of the above at frame N");
        eprintln!("  --trace-mapper     Log all mapper register writes");
        eprintln!("  --trace-pc         Log every PC change");
        eprintln!();
        eprintln!("Example:");
        eprintln!(
            "  {} \"rom.nes\" --frames 120 --dump-all 30 --dump-frame 60",
            args[0]
        );
        std::process::exit(1);
    }

    let rom_path = &args[1];
    let opts = parse_opts(&args[2..]);

    let data = std::fs::read(rom_path).expect("failed to read ROM");
    let rom = Rom::new(&data).expect("invalid iNES ROM");
    let rom_name = std::path::Path::new(rom_path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "rom".to_string());

    eprintln!(
        "ROM: {} (mapper {}, PRG={}KB, CHR={}KB{})",
        rom_name,
        rom.mapper_id,
        rom.prg.len() / 1024,
        rom.chr.len() / 1024,
        if rom.has_chr_ram { " + CHR-RAM" } else { "" }
    );

    let mut cpu = CpuRp2a03::new(0);
    let mut bus = Bus::new(rom.create_mapper());
    reset(&mut cpu, &mut bus);

    let max_frames = opts.frames;
    let mut frame_count = 0u32;
    let mut total_cycles = 0u64;

    // Trace PC if requested
    let trace_pc = opts.trace_pc;

    // Track previous PC for detecting infinite loops
    let mut prev_pc = 0u16;
    let mut same_pc_count = 0u32;

    loop {
        let cycles = tick(&mut cpu, &mut bus);
        total_cycles += cycles as u64;

        // ─── PC tracing ────────────────────────────────────────────
        if trace_pc && frame_count < 5 {
            eprintln!(
                "PC=${:04X} A=${:02X} X=${:02X} Y=${:02X} SP=${:02X} SR=${:02X}",
                cpu.pc(),
                cpu.a(),
                cpu.x(),
                cpu.y(),
                cpu.st(),
                cpu.sr()
            );
        }

        // ─── Infinite loop detection ───────────────────────────────
        if cpu.pc() == prev_pc {
            same_pc_count += 1;
            if same_pc_count > 5_000_000 {
                eprintln!(
                    "⚠ HANG DETECTED at PC=${:04X} (same PC for 5M instructions)",
                    cpu.pc()
                );
                break;
            }
        } else {
            same_pc_count = 0;
            prev_pc = cpu.pc();
        }

        // ─── Frame complete ────────────────────────────────────────
        if bus.ppu.frame_complete {
            bus.ppu.frame_complete = false;

            let f = frame_count as i32;

            // Dump at specific frame
            if opts.dump_frame == f {
                let path = format!("{}_frame{}.ppm", rom_name, f);
                debug::save_frame_ppm(&bus.ppu.frame, &path);
                eprintln!("📸 Saved {}", path);
            }
            if opts.dump_ppu == f || opts.dump_all == f {
                debug::dump_ppu_state(&bus.ppu);
            }
            if opts.dump_palette == f || opts.dump_all == f {
                debug::dump_palette(&bus.ppu);
            }
            if opts.dump_oam == f || opts.dump_all == f {
                debug::dump_oam(&bus.ppu);
            }
            if opts.dump_nt == f || opts.dump_all == f {
                for nt in 0..2 {
                    debug::dump_nametable(&bus.ppu.vram, nt);
                    debug::dump_attribute_table(&bus.ppu.vram, nt);
                }
            }
            if opts.dump_chr == f || opts.dump_all == f {
                // Determine CHR source from mapper
                if !bus.mapper.has_chr_ram() {
                    // Read from mapper directly for CHR ROM dumps
                    eprintln!("(CHR-ROM dump not available through PPU, see mapper registers)");
                }
            }
            if opts.diag == f || opts.dump_all == f {
                debug::quick_diagnostics(&bus.ppu, &bus.mapper, &rom_name);
            }
            if opts.dump_mmc5 == f || opts.dump_all == f {
                debug::dump_mmc5_state(&bus.mapper);
                debug::dump_mmc5_exram(&bus.mapper, 64);
            }

            frame_count += 1;
            if frame_count % 10 == 0 || frame_count <= 5 {
                eprintln!(
                    "Frame {:>4} — PC=${:04X}  cycles={}",
                    f,
                    cpu.pc(),
                    total_cycles
                );
            }
            if frame_count >= max_frames {
                eprintln!("━━━ Reached {} frames ━━━", max_frames);
                break;
            }
        }
    }

    eprintln!(
        "\nDone: {} frames, {} CPU cycles, final PC=${:04X}",
        frame_count,
        total_cycles,
        cpu.pc()
    );
}

// ─── CLI Option Parsing ──────────────────────────────────────────────

struct Opts {
    frames: u32,
    dump_frame: i32,
    dump_ppu: i32,
    dump_palette: i32,
    dump_oam: i32,
    dump_nt: i32,
    dump_chr: i32,
    diag: i32,
    dump_mmc5: i32,
    dump_all: i32,
    trace_mapper: bool,
    trace_pc: bool,
}

fn parse_opts(args: &[String]) -> Opts {
    let mut opts = Opts {
        frames: 60,
        dump_frame: -1,
        dump_ppu: -1,
        dump_palette: -1,
        dump_oam: -1,
        dump_nt: -1,
        dump_chr: -1,
        diag: -1,
        dump_mmc5: -1,
        dump_all: -1,
        trace_mapper: false,
        trace_pc: false,
    };

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--frames" if i + 1 < args.len() => {
                opts.frames = args[i + 1].parse().unwrap_or(60);
                i += 1;
            }
            "--dump-frame" if i + 1 < args.len() => {
                opts.dump_frame = args[i + 1].parse().unwrap_or(0);
                i += 1;
            }
            "--dump-ppu" if i + 1 < args.len() => {
                opts.dump_ppu = args[i + 1].parse().unwrap_or(0);
                i += 1;
            }
            "--dump-palette" if i + 1 < args.len() => {
                opts.dump_palette = args[i + 1].parse().unwrap_or(0);
                i += 1;
            }
            "--dump-oam" if i + 1 < args.len() => {
                opts.dump_oam = args[i + 1].parse().unwrap_or(0);
                i += 1;
            }
            "--dump-nt" if i + 1 < args.len() => {
                opts.dump_nt = args[i + 1].parse().unwrap_or(0);
                i += 1;
            }
            "--dump-chr" if i + 1 < args.len() => {
                opts.dump_chr = args[i + 1].parse().unwrap_or(0);
                i += 1;
            }
            "--diag" if i + 1 < args.len() => {
                opts.diag = args[i + 1].parse().unwrap_or(0);
                i += 1;
            }
            "--dump-mmc5" if i + 1 < args.len() => {
                opts.dump_mmc5 = args[i + 1].parse().unwrap_or(0);
                i += 1;
            }
            "--dump-all" if i + 1 < args.len() => {
                opts.dump_all = args[i + 1].parse().unwrap_or(0);
                i += 1;
            }
            "--trace-mapper" => opts.trace_mapper = true,
            "--trace-pc" => opts.trace_pc = true,
            _ => {
                eprintln!("⚠ Unknown option: {}", args[i]);
            }
        }
        i += 1;
    }
    opts
}
