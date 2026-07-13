use nes::bus::Bus;
use nes::cpu::Cpu6502;
use nes::rom::Rom;
use nes::{reset, tick};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <rom.nes>", args[0]);
        std::process::exit(1);
    }

    let data = std::fs::read(&args[1]).expect("failed to read ROM");
    let rom = Rom::new(&data).expect("invalid iNES ROM");

    let mut cpu = Cpu6502::new(0);
    let mut bus = Bus::new(&rom.prg, &rom.chr);
    reset(&mut cpu, &mut bus);

    let mut total_cycles = 0u64;
    loop {
        let cycles = tick(&mut cpu, &mut bus);
        total_cycles += cycles as u64;

        if bus.ppu.frame_complete {
            bus.ppu.frame_complete = false;
            println!(
                "Frame complete — {} CPU cycles, PC = ${:04X}",
                total_cycles,
                cpu.pc()
            );
        }
    }
}
