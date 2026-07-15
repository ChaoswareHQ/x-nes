#![allow(
    clippy::missing_safety_doc,
    clippy::module_name_repetitions,
    clippy::similar_names,
    dead_code
)]

use core::mem::MaybeUninit;
use core::ptr::{addr_of, addr_of_mut};
use core::slice;

use crate::bus::Bus;
use crate::cpu::CpuRp2a03;
use crate::rom::Rom;
use crate::{reset, tick};

struct NesEmulator {
    bus: Bus,
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

        self.cpu = CpuRp2a03::new(0x0000);
        self.bus = Bus::new(rom.create_mapper());
        reset(&mut self.cpu, &mut self.bus);
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
        self.bus.ppu_read_mapped(addr)
    }

    #[inline(always)]
    fn write_ppu(&mut self, addr: u16, val: u8) {
        self.bus.ppu_write_mapped(addr, val);
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
            *b = self.bus.ppu_read_mapped(addr.wrapping_add(i as u16));
        }
    }
    fn write_ppu_block(&mut self, addr: u16, src: &[u8]) {
        for (i, &b) in src.iter().enumerate() {
            self.bus.ppu_write_mapped(addr.wrapping_add(i as u16), b);
        }
    }

    #[inline(always)]
    fn frame_ptr(&self) -> *const u8 {
        self.bus.ppu.frame.as_ptr()
    }

    #[inline(always)]
    fn audio_samples(&self) -> (*const f32, usize) {
        (
            self.bus.apu.audio_samples.as_ptr(),
            self.bus.apu.sample_count,
        )
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
    unsafe {
        addr_of_mut!(EMU).write(MaybeUninit::zeroed());
    }
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
