#![allow(
    clippy::missing_safety_doc,
    clippy::module_name_repetitions,
    clippy::similar_names
)]

use core::mem::{transmute, MaybeUninit};
use core::ptr::{addr_of, addr_of_mut};
use core::slice;

use crate::bus::Bus;
use crate::cpu::CpuRp2a03;
use crate::rom::Rom;
use crate::{reset, tick};

const MAX_PRG_SIZE: usize = 0x8000;
const MAX_CHR_SIZE: usize = 0x2000;

struct NesEmulator {
    prg: [u8; MAX_PRG_SIZE],
    chr: [u8; MAX_CHR_SIZE],
    prg_size: usize,
    chr_size: usize,
    bus: Bus<'static>,
    cpu: CpuRp2a03,
}

static mut EMU: MaybeUninit<NesEmulator> = MaybeUninit::uninit();

impl NesEmulator {
    #[inline(always)]
    fn load(&mut self, data: &[u8]) -> bool {
        let rom = match Rom::new(data) {
            Some(r) => r,
            None => return false,
        };

        let prg_len = rom.prg_size.min(MAX_PRG_SIZE);
        let chr_len = rom.chr_size.min(MAX_CHR_SIZE);

        self.prg[..prg_len].copy_from_slice(&rom.prg[..prg_len]);
        self.chr[..chr_len].copy_from_slice(&rom.chr[..chr_len]);
        self.prg_size = prg_len;
        self.chr_size = chr_len;

        let prg_slice: &'static [u8] = unsafe { transmute(&self.prg[..prg_len]) };
        let chr_slice: &'static [u8] = unsafe { transmute(&self.chr[..chr_len]) };

        let mut bus = Bus::new(prg_slice, chr_slice, rom.mirroring);
        let mut cpu = CpuRp2a03::new(0x0000);
        reset(&mut cpu, &mut bus);

        self.bus = bus;
        self.cpu = cpu;
        true
    }

    #[inline(always)]
    fn reset(&mut self) {
        reset(&mut self.cpu, &mut self.bus);
    }

    #[inline(always)]
    fn step(&mut self) -> u8 {
        tick(&mut self.cpu, &mut self.bus)
    }

    #[inline(always)]
    fn run_frame(&mut self) {
        loop {
            self.step();
            if self.bus.ppu.frame_complete {
                self.bus.ppu.frame_complete = false;
                break;
            }
        }
    }

    #[inline(always)]
    fn read_cpu(&mut self, addr: u16) -> u8 {
        self.bus.read(addr)
    }
    #[inline(always)]
    fn write_cpu(&mut self, addr: u16, val: u8) {
        self.bus.write(addr, val);
    }
    #[inline(always)]
    fn read_ppu(&mut self, addr: u16) -> u8 {
        self.bus.ppu.ppu_read(addr)
    }
    #[inline(always)]
    fn write_ppu(&mut self, addr: u16, val: u8) {
        self.bus.ppu.ppu_write(addr, val);
    }

    fn read_cpu_block(&mut self, addr: u16, dst: &mut [u8]) {
        for (i, b) in dst.iter_mut().enumerate() {
            *b = self.bus.read(addr.wrapping_add(i as u16));
        }
    }
    fn write_cpu_block(&mut self, addr: u16, src: &[u8]) {
        for (i, &b) in src.iter().enumerate() {
            self.bus.write(addr.wrapping_add(i as u16), b);
        }
    }
    fn read_ppu_block(&mut self, addr: u16, dst: &mut [u8]) {
        for (i, b) in dst.iter_mut().enumerate() {
            *b = self.bus.ppu.ppu_read(addr.wrapping_add(i as u16));
        }
    }
    fn write_ppu_block(&mut self, addr: u16, src: &[u8]) {
        for (i, &b) in src.iter().enumerate() {
            self.bus.ppu.ppu_write(addr.wrapping_add(i as u16), b);
        }
    }

    #[inline(always)]
    fn prg_mut_ptr(&mut self) -> *mut u8 {
        self.prg.as_mut_ptr()
    }
    #[inline(always)]
    fn chr_mut_ptr(&mut self) -> *mut u8 {
        self.chr.as_mut_ptr()
    }
    #[inline(always)]
    fn prg_size(&self) -> usize {
        self.prg_size
    }
    #[inline(always)]
    fn chr_size(&self) -> usize {
        self.chr_size
    }

    #[inline(always)]
    fn frame_ptr(&self) -> *const u8 {
        self.bus.ppu.frame.as_ptr()
    }

    #[inline(always)]
    fn audio_samples(&self) -> (*const f32, usize) {
        (self.bus.apu.audio_samples.as_ptr(), self.bus.apu.sample_count)
    }
    #[inline(always)]
    fn clear_audio(&mut self) {
        self.bus.apu.sample_count = 0;
    }
    #[inline(always)]
    fn set_sample_rate(&mut self, rate: f32) {
        self.bus.apu.set_sample_rate(rate as f64);
    }

    #[inline(always)]
    fn poll_nmi(&mut self) -> bool {
        self.bus.poll_nmi()
    }

    #[inline(always)]
    fn set_button(&mut self, button: u8, pressed: bool) {
        let pad = &mut self.bus.pad1;
        match button {
            b'A' => pad.a = pressed,
            b'B' => pad.b = pressed,
            b'S' => pad.select = pressed,
            b'T' => pad.start = pressed,
            b'U' => pad.up = pressed,
            b'D' => pad.down = pressed,
            b'L' => pad.left = pressed,
            b'R' => pad.right = pressed,
            _ => {}
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_init() -> i32 {
    unsafe { addr_of_mut!(EMU).write(MaybeUninit::zeroed()); }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_load_rom(ptr: *const u8, len: usize) -> i32 {
    if ptr.is_null() || len == 0 {
        return -1;
    }
    let data = unsafe { slice::from_raw_parts(ptr, len) };
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    if emu.load(data) { 0 } else { -2 }
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_reset() {
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    emu.reset();
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_step() -> u8 {
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    emu.step()
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_run_frame() {
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    emu.run_frame();
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_read_cpu(addr: u16) -> u8 {
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    emu.read_cpu(addr)
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_write_cpu(addr: u16, val: u8) {
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    emu.write_cpu(addr, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_read_ppu(addr: u16) -> u8 {
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    emu.read_ppu(addr)
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_write_ppu(addr: u16, val: u8) {
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    emu.write_ppu(addr, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_read_cpu_block(addr: u16, dst: *mut u8, len: usize) {
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    let slice = unsafe { slice::from_raw_parts_mut(dst, len) };
    emu.read_cpu_block(addr, slice);
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_write_cpu_block(addr: u16, src: *const u8, len: usize) {
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    let slice = unsafe { slice::from_raw_parts(src, len) };
    emu.write_cpu_block(addr, slice);
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_read_ppu_block(addr: u16, dst: *mut u8, len: usize) {
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    let slice = unsafe { slice::from_raw_parts_mut(dst, len) };
    emu.read_ppu_block(addr, slice);
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_write_ppu_block(addr: u16, src: *const u8, len: usize) {
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    let slice = unsafe { slice::from_raw_parts(src, len) };
    emu.write_ppu_block(addr, slice);
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_get_prg_ptr() -> *mut u8 {
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    emu.prg_mut_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_get_prg_size() -> usize {
    let emu = unsafe { addr_of!(EMU).cast::<NesEmulator>().as_ref().unwrap() };
    emu.prg_size()
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_get_chr_ptr() -> *mut u8 {
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    emu.chr_mut_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_get_chr_size() -> usize {
    let emu = unsafe { addr_of!(EMU).cast::<NesEmulator>().as_ref().unwrap() };
    emu.chr_size()
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_get_frame_ptr() -> *const u8 {
    let emu = unsafe { addr_of!(EMU).cast::<NesEmulator>().as_ref().unwrap() };
    emu.frame_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_get_audio_ptr() -> *const f32 {
    let emu = unsafe { addr_of!(EMU).cast::<NesEmulator>().as_ref().unwrap() };
    let (ptr, _) = emu.audio_samples();
    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_get_audio_count() -> usize {
    let emu = unsafe { addr_of!(EMU).cast::<NesEmulator>().as_ref().unwrap() };
    let (_, count) = emu.audio_samples();
    count
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_clear_audio() {
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    emu.clear_audio();
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_set_sample_rate(rate: f32) {
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    emu.set_sample_rate(rate);
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_poll_nmi() -> u8 {
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    if emu.poll_nmi() { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn nes_set_button(button: u8, pressed: u8) {
    let emu = unsafe { addr_of_mut!(EMU).cast::<NesEmulator>().as_mut().unwrap() };
    emu.set_button(button, pressed != 0);
}