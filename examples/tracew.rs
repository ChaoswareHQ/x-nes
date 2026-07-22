/// Trace all writes to PPU registers ($2000-$2007) and MMC5 registers ($5000-$5FFF).
use nes::bus::Bus;
use nes::cpu::CpuRp2a03;
use nes::rom::Rom;
use nes::{reset, tick};
use std::{env, fs};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <rom.nes>", args[0]);
        return;
    }

    let data = fs::read(&args[1]).expect("failed to read ROM");
    let rom = Rom::new(&data).expect("invalid iNES ROM");

    let mut cpu = CpuRp2a03::new(0);
    let mut bus = Bus::new(rom.create_mapper());
    reset(&mut cpu, &mut bus);

    let mut frame = 0u32;
    let mut last_pc_2000 = 0u16;
    let mut wrote_2000 = false;

    println!("Writes to PPU and MMC5 registers (first 60 frames):");
    println!("===================================================");

    loop {
        let pc_before = cpu.pc();
        tick(&mut cpu, &mut bus);
        let pc_after = cpu.pc();

        if bus.ppu.frame_complete {
            bus.ppu.frame_complete = false;
            frame += 1;
            if frame % 10 == 0 {
                eprintln!("Frame {} — PC=${:04X}", frame, pc_after);
            }
            if frame > 60 {
                break;
            }
        }
    }
}
