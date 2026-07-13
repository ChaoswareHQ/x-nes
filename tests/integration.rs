use nes::bus::Bus;
use nes::cpu::CpuRp2a03;
use nes::rom::Rom;
use nes::{reset, tick};

fn load_rom(data: &[u8]) -> (CpuRp2a03, Bus<'static>) {
    let rom = Rom::new(data).expect("invalid ROM");
    let prg: &'static [u8] = Box::leak(rom.prg.to_vec().into_boxed_slice());
    let chr: &'static [u8] = Box::leak(rom.chr.to_vec().into_boxed_slice());
    let mut cpu = CpuRp2a03::new(0);
    let mut bus = Bus::new(prg, chr, rom.mirroring);
    reset(&mut cpu, &mut bus);
    (cpu, bus)
}

#[test]
fn cpu_initial_state() {
    let cpu = CpuRp2a03::new(0x8000);
    assert_eq!(cpu.pc(), 0x8000);
    assert_eq!(cpu.a(), 0);
    assert_eq!(cpu.x(), 0);
    assert_eq!(cpu.y(), 0);
    assert_eq!(cpu.st(), 0);
    assert_eq!(cpu.sr(), 0);
}

#[test]
fn cpu_set_get_registers() {
    let mut cpu = CpuRp2a03::new(0);
    cpu.set_pc(0x1234);
    cpu.set_a(0xAB);
    cpu.set_x(0xCD);
    cpu.set_y(0xEF);
    cpu.set_st(0xFE);
    cpu.set_sr(0xFF);
    assert_eq!(cpu.pc(), 0x1234);
    assert_eq!(cpu.a(), 0xAB);
    assert_eq!(cpu.x(), 0xCD);
    assert_eq!(cpu.y(), 0xEF);
    assert_eq!(cpu.st(), 0xFE);
    assert_eq!(cpu.sr(), 0xFF);
}

#[test]
fn cpu_flags() {
    use nes::cpu::FLAG_CARRY;
    let mut cpu = CpuRp2a03::new(0);
    assert!(!cpu.get_flag(FLAG_CARRY));
    cpu.set_flag(FLAG_CARRY, true);
    assert!(cpu.get_flag(FLAG_CARRY));
    cpu.set_flag(FLAG_CARRY, false);
    assert!(!cpu.get_flag(FLAG_CARRY));
}

#[test]
fn cpu_sign_zero_flags() {
    let mut cpu = CpuRp2a03::new(0);
    cpu.set_sign(0x80);
    assert!(cpu.get_flag(nes::cpu::FLAG_NEGATIVE));
    cpu.set_sign(0x7F);
    assert!(!cpu.get_flag(nes::cpu::FLAG_NEGATIVE));
    cpu.set_zero(0x00);
    assert!(cpu.get_flag(nes::cpu::FLAG_ZERO));
    cpu.set_zero(0x01);
    assert!(!cpu.get_flag(nes::cpu::FLAG_ZERO));
}

#[test]
fn cpu_byte_serialization() {
    let mut cpu = CpuRp2a03::new(0xABCD);
    cpu.set_a(0x12);
    cpu.set_x(0x34);
    cpu.set_y(0x56);
    cpu.set_st(0x78);
    cpu.set_sr(0x9A);
    let bytes = cpu.as_bytes();
    let restored = CpuRp2a03::from_bytes(bytes);
    assert_eq!(restored.pc(), 0xABCD);
    assert_eq!(restored.a(), 0x12);
    assert_eq!(restored.x(), 0x34);
    assert_eq!(restored.y(), 0x56);
    assert_eq!(restored.st(), 0x78);
    assert_eq!(restored.sr(), 0x9A);
}

fn make_nrom_rom() -> Vec<u8> {
    let mut data = vec![0x4E, 0x45, 0x53, 0x1A, 1, 0, 0, 0];
    data.resize(16, 0);
    let mut prg = vec![0xEAu8; 0x4000];
    // Set PPUMASK via absolute store at boot
    prg[0x0000] = 0xA9; // LDA #$1E
    prg[0x0001] = 0x1E;
    prg[0x0002] = 0x8D; // STA $2001
    prg[0x0003] = 0x01;
    prg[0x0004] = 0x20;
    prg[0x0005] = 0xA9; // LDA #$80
    prg[0x0006] = 0x80;
    prg[0x0007] = 0x8D; // STA $2000
    prg[0x0008] = 0x00;
    prg[0x0009] = 0x20;
    prg[0x000A] = 0x4C; // JMP $800A
    prg[0x000B] = 0x0A;
    prg[0x000C] = 0x80;
    prg[0x3FFC] = 0x00;
    prg[0x3FFD] = 0x80;
    data.extend(&prg);
    data
}

#[test]
fn bus_ram_read_write() {
    let (_, mut bus) = load_rom(&make_nrom_rom());
    bus.write(0x0000, 0x42);
    assert_eq!(bus.read(0x0000), 0x42);
    assert_eq!(bus.read(0x0800), 0x42);
    assert_eq!(bus.read(0x1000), 0x42);
    assert_eq!(bus.read(0x1800), 0x42);
}

#[test]
fn bus_prg_read() {
    let (_, mut bus) = load_rom(&make_nrom_rom());
    assert_eq!(bus.read(0x8000), 0xA9);
    assert_eq!(bus.read(0xC000), 0xA9);
}

#[test]
fn execute_lda_imm() {
    let (mut cpu, mut bus) = load_rom(&make_nrom_rom());
    let cy = tick(&mut cpu, &mut bus);
    assert_eq!(cpu.a(), 0x1E);
    assert_eq!(cy, 2);
}

#[test]
fn execute_ppu_write() {
    let (mut cpu, mut bus) = load_rom(&make_nrom_rom());
    tick(&mut cpu, &mut bus); // LDA #$1E
    tick(&mut cpu, &mut bus); // STA $2001
    assert_eq!(bus.ppu.mask, 0x1E);
}

#[test]
fn execute_ppu_ctrl() {
    let (mut cpu, mut bus) = load_rom(&make_nrom_rom());
    tick(&mut cpu, &mut bus); // LDA #$1E
    tick(&mut cpu, &mut bus); // STA $2001
    tick(&mut cpu, &mut bus); // LDA #$80
    tick(&mut cpu, &mut bus); // STA $2000
    assert_eq!(bus.ppu.ctrl, 0x80);
}

#[test]
fn render_nrom_frame() {
    let (mut cpu, mut bus) = load_rom(&make_nrom_rom());
    bus.ppu.palette[0] = 0x16;
    bus.ppu.palette[1] = 0x2A;
    while !bus.ppu.frame_complete {
        tick(&mut cpu, &mut bus);
    }
    bus.ppu.frame_complete = false;
    // Check pixel outside sprite 0 (which is at 0,0)
    let mid = bus.ppu.frame[10 * 256 + 10];
    assert_eq!(mid, 0x16, "pixel outside sprite should use bg colour");
}

#[test]
fn ppu_frame_complete() {
    let (_, mut bus) = load_rom(&make_nrom_rom());
    for _ in 0..(262 * 341) {
        bus.ppu.tick();
    }
    assert!(bus.ppu.frame_complete);
}

#[test]
fn reset_sets_pc() {
    let (cpu, _) = load_rom(&make_nrom_rom());
    assert_eq!(cpu.pc(), 0x8000);
}

#[test]
fn gamepad_write_strobe() {
    let mut pad = nes::gamepad::Gamepad::new();

    pad.a = true;
    pad.write(1);
    assert_eq!(pad.read(), 1);
    assert_eq!(pad.read(), 1);

    pad.a = true;
    pad.b = true;
    pad.select = true;
    pad.start = true;
    pad.up = true;
    pad.down = true;
    pad.left = true;
    pad.right = true;
    pad.write(1);
    pad.write(0);
    assert_eq!(pad.read(), 1);
    assert_eq!(pad.read(), 1);
    assert_eq!(pad.read(), 1);
    assert_eq!(pad.read(), 1);
    assert_eq!(pad.read(), 1);
    assert_eq!(pad.read(), 1);
    assert_eq!(pad.read(), 1);
    assert_eq!(pad.read(), 1);
    assert_eq!(pad.read(), 1);

    let mut pad = nes::gamepad::Gamepad::new();
    pad.a = true;
    pad.write(1);
    pad.write(0);
    assert_eq!(pad.read(), 1);
    assert_eq!(pad.read(), 0);
    assert_eq!(pad.read(), 0);
    assert_eq!(pad.read(), 0);
}

#[test]
#[cfg(feature = "std")]
fn download_nova_and_check_ctrl() {
    let resp = ureq::get(
        "https://github.com/NovaSquirrel/NovaTheSquirrel/releases/download/v1.0.6a/nova.nes",
    )
    .call()
    .unwrap();
    let data = resp.into_body().read_to_vec().unwrap();
    let (mut cpu, mut bus) = load_rom(&data);

    for _ in 0..(1000 * 100) {
        tick(&mut cpu, &mut bus);
    }
    assert!(bus.ppu.ctrl & 0x80 != 0, "NMI should be enabled");
}
