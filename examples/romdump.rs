/// Dump code through the mapper.
use nes::bus::Bus;
use nes::rom::Rom;
use std::{env, fs};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <rom.nes>", args[0]);
        return;
    }

    let data = fs::read(&args[1]).expect("failed to read ROM");
    let rom = Rom::new(&data).expect("invalid iNES ROM");
    eprintln!(
        "ROM: mapper={} PRG={}KB CHR={}KB",
        rom.mapper_id,
        rom.prg.len() / 1024,
        rom.chr.len() / 1024
    );

    let mut bus = Bus::new(rom.create_mapper());

    for &start in &[0xAA10u16, 0xAA23u16, 0x801Eu16, 0xA234u16] {
        eprintln!("\n=== ${:04X} ===", start);
        for i in 0..64 {
            if i % 16 == 0 {
                eprint!("  ");
            }
            eprint!("{:02X} ", bus.mapper.cpu_read(start.wrapping_add(i)));
            if (i + 1) % 16 == 0 {
                eprintln!();
            }
        }
    }
}
