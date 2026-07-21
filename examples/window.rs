use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use nes::bus::Bus;
use nes::cpu::CpuRp2a03;
use nes::rom::Rom;
use nes::{reset, tick};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use gilrs::{Button, Gilrs};
use ringbuf::HeapRb;
use ringbuf::traits::{Consumer, Producer, Split};
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes};

const NES_W: u32 = 256;
const NES_H: u32 = 240;
const NES_PAR: f64 = 8.0 / 7.0;
const SCALE: u32 = 3;
const DEFAULT_ROM: &str =
    "https://github.com/NovaSquirrel/NovaTheSquirrel/releases/download/v1.0.6a/nova.nes";

// Actual NES frame duration: 89,342 PPU dots / (3 PPU dots per CPU cycle * 1,789,773 Hz)
const NES_FRAME_NS: u64 = 16_639_000;

static PALETTE: [u32; 64] = [
    0xFF545454, 0xFF001E74, 0xFF080090, 0xFF440088, 0xFF7C005C, 0xFFA4001C, 0xFFA80000, 0xFF880000,
    0xFF5C2800, 0xFF284400, 0xFF005400, 0xFF005030, 0xFF004444, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFFB4B4B4, 0xFF0C54C4, 0xFF303CD8, 0xFF742CC4, 0xFFAC1898, 0xFFD8004C, 0xFFDC0800, 0xFFBC3000,
    0xFF805000, 0xFF486800, 0xFF107800, 0xFF007444, 0xFF00686C, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFFFCFCFC, 0xFF64B0FC, 0xFF9090FC, 0xFFC87CFC, 0xFFFC74FC, 0xFFFC74B8, 0xFFFC7870, 0xFFFC9838,
    0xFFF0B800, 0xFFBCD000, 0xFF84DC48, 0xFF58D878, 0xFF44D0A8, 0xFF000000, 0xFF000000, 0xFF000000,
    0xFFFCFCFC, 0xFFC0E4FC, 0xFFD0D4FC, 0xFFE8CCFC, 0xFFFCC8FC, 0xFFFCC4E0, 0xFFFCC8B8, 0xFFFCD4A0,
    0xFFFCE090, 0xFFE4EC88, 0xFFC8F090, 0xFFA8F0A8, 0xFFB0ECC8, 0xFF000000, 0xFF000000, 0xFF000000,
];

fn nes_colour(index: u8) -> u32 {
    PALETTE[(index & 0x3F) as usize]
}

fn scale_frame(src: &[u8; (NES_W * NES_H) as usize], dst: &mut [u32], dw: u32, dh: u32) {
    for y in 0..dh {
        for x in 0..dw {
            let sx = x * NES_W / dw;
            let sy = y * NES_H / dh;
            dst[(y * dw + x) as usize] = nes_colour(src[(sy * NES_W + sx) as usize]);
        }
    }
}

fn load_rom(path_or_url: &str) -> Vec<u8> {
    if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
        eprintln!("Downloading {}...", path_or_url);
        let resp = ureq::get(path_or_url)
            .call()
            .expect("failed to download ROM");
        let data = resp.into_body().read_to_vec().expect("failed to read body");
        eprintln!("Downloaded {} bytes", data.len());
        data
    } else {
        std::fs::read(path_or_url).expect("failed to read ROM file")
    }
}

struct App {
    cpu: CpuRp2a03,
    bus: Bus,
    window: Option<std::rc::Rc<Window>>,
    ctx: Option<Context<std::rc::Rc<Window>>>,
    surface: Option<Surface<std::rc::Rc<Window>, std::rc::Rc<Window>>>,
    frame_timer: Instant,
    frame_dur: Duration,
    acc: Duration,
    gilrs: Gilrs,
    audio_stream: Option<cpal::Stream>,
    audio_tx: Option<
        ringbuf::CachingProd<std::sync::Arc<ringbuf::SharedRb<ringbuf::storage::Heap<i16>>>>,
    >,
}

impl App {
    fn new(rom_path: Option<String>) -> Self {
        let path = rom_path.as_deref().unwrap_or(DEFAULT_ROM);
        let data = load_rom(path);
        let rom = Rom::new(&data).expect("invalid iNES ROM");

        let mut cpu = CpuRp2a03::new(0);
        let mut bus = Bus::new(rom.create_mapper());
        reset(&mut cpu, &mut bus);

        Self {
            cpu,
            bus,
            gilrs: Gilrs::new().expect("failed to initialize gilrs"),
            window: None,
            ctx: None,
            surface: None,
            frame_timer: Instant::now(),
            frame_dur: Duration::from_nanos(NES_FRAME_NS),
            acc: Duration::new(0, 0),
            audio_stream: None,
            audio_tx: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let w = (NES_W as f64 * NES_PAR * SCALE as f64).round() as u32;
        let h = NES_H * SCALE;

        let window = event_loop
            .create_window(
                WindowAttributes::default()
                    .with_title("x-nes")
                    .with_inner_size(LogicalSize::new(w, h)),
            )
            .expect("failed to create window");

        let rc = std::rc::Rc::new(window);
        let ctx = Context::new(rc.clone()).expect("failed to create softbuffer context");
        let surface = Surface::new(&ctx, rc.clone()).expect("failed to create softbuffer surface");

        self.window = Some(rc);
        self.ctx = Some(ctx);
        self.surface = Some(surface);
        self.frame_timer = Instant::now();
        event_loop.set_control_flow(ControlFlow::Poll);

        // Init audio
        if self.audio_stream.is_none() {
            let host = cpal::default_host();
            if let Some(device) = host.default_output_device() {
                eprintln!("Audio device found, initializing...");
                // Use the device's default config instead of hardcoding
                match device.default_output_config() {
                    Ok(supported) => {
                        let sample_rate = supported.sample_rate();
                        let channels = supported.channels();
                        eprintln!("Audio config: {}Hz, {} channels", sample_rate, channels);

                        // Update APU sample rate to match device
                        self.bus.apu.set_sample_rate(sample_rate);

                        // Use a larger ring buffer to tolerate timing jitter
                        let rb = HeapRb::<i16>::new(32768);
                        let (mut prod, mut cons) = rb.split();
                        let ch = channels as usize;

                        // Pre-fill the ring buffer with ~2 frames of silence
                        // to prevent underruns while the emulator gets going
                        let frames_to_fill = (sample_rate as f64 / 60.0 * 2.0) as usize;
                        for _ in 0..frames_to_fill {
                            let _ = prod.try_push(0);
                        }
                        eprintln!("Pre-filled buffer with {} samples", frames_to_fill);

                        let err_fn = |err| eprintln!("audio stream error: {}", err);

                        let config: cpal::StreamConfig = supported.into();
                        let stream = device.build_output_stream(
                            config,
                            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                                // Fill all channels with the same mono sample
                                for frame in data.chunks_mut(ch) {
                                    let s = cons.try_pop().unwrap_or(0) as f32 / 32767.0;
                                    for sample in frame.iter_mut() {
                                        *sample = s;
                                    }
                                }
                            },
                            err_fn,
                            None,
                        );

                        match stream {
                            Ok(stream) => {
                                match stream.play() {
                                    Ok(_) => eprintln!("Audio stream started successfully"),
                                    Err(e) => eprintln!("Failed to play audio stream: {}", e),
                                }
                                self.audio_stream = Some(stream);
                                self.audio_tx = Some(prod);
                            }
                            Err(e) => eprintln!("Failed to build audio stream: {}", e),
                        }
                    }
                    Err(e) => eprintln!("Failed to get default output config: {}", e),
                }
            } else {
                eprintln!("No audio output device found");
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(surface) = &mut self.surface {
                    surface
                        .resize(
                            NonZeroU32::new(size.width.max(1)).unwrap(),
                            NonZeroU32::new(size.height.max(1)).unwrap(),
                        )
                        .unwrap();
                }
            }
            WindowEvent::KeyboardInput { event, .. } if !event.repeat => {
                let pressed = event.state == ElementState::Pressed;
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyZ) => self.bus.pad1.b = pressed,
                    PhysicalKey::Code(KeyCode::KeyX) => self.bus.pad1.a = pressed,
                    PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => {
                        self.bus.pad1.select = pressed
                    }
                    PhysicalKey::Code(KeyCode::Enter) => self.bus.pad1.start = pressed,
                    PhysicalKey::Code(KeyCode::ArrowUp) => self.bus.pad1.up = pressed,
                    PhysicalKey::Code(KeyCode::ArrowDown) => self.bus.pad1.down = pressed,
                    PhysicalKey::Code(KeyCode::ArrowLeft) => self.bus.pad1.left = pressed,
                    PhysicalKey::Code(KeyCode::ArrowRight) => self.bus.pad1.right = pressed,
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(surface) = &mut self.surface {
                    let rc = self.window.as_ref().unwrap();
                    let size = rc.inner_size();
                    let dw = size.width.max(1);
                    let dh = size.height.max(1);
                    let mut buf = vec![0u32; (dw * dh) as usize];
                    scale_frame(&self.bus.ppu.frame, &mut buf, dw, dh);

                    if let Ok(mut fb) = surface.buffer_mut() {
                        let slice = fb.as_mut();
                        let n = slice.len().min(buf.len());
                        slice[..n].copy_from_slice(&buf[..n]);
                        let _ = fb.present();
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        self.acc += now - self.frame_timer;
        self.frame_timer = now;

        if self.acc > Duration::from_millis(100) {
            self.acc = Duration::from_millis(100);
        }

        let _ticked = false;
        while self.acc >= self.frame_dur {
            while !self.bus.ppu.frame_complete {
                tick(&mut self.cpu, &mut self.bus);
            }
            self.bus.ppu.frame_complete = false;
            self.acc -= self.frame_dur;

            // Push audio samples (discard if buffer full — will log occasionally)
            if let Some(tx) = &mut self.audio_tx {
                let n = self.bus.apu.sample_count;
                if n > 0 {
                    let pushed = tx.push_slice(&self.bus.apu.audio_samples[..n]);
                    if pushed < n && pushed == 0 {
                        // Buffer full — samples dropped. This should be rare.
                        eprintln!("audio buffer full, dropped {} samples", n);
                    }
                }
                self.bus.apu.sample_count = 0;
            }
        }

        // Poll game controller
        while let Some(gilrs::Event {
            id: _,
            event,
            time: _,
        }) = self.gilrs.next_event()
        {
            match event {
                gilrs::EventType::ButtonChanged(button, val, _) => {
                    let pressed = val > 0.5;
                    match button {
                        Button::South => self.bus.pad1.a = pressed, // A
                        Button::East => self.bus.pad1.b = pressed,  // B
                        Button::West => self.bus.pad1.b = pressed,  // X -> B
                        Button::North => self.bus.pad1.a = pressed, // Y -> A
                        Button::DPadUp => self.bus.pad1.up = pressed,
                        Button::DPadDown => self.bus.pad1.down = pressed,
                        Button::DPadLeft => self.bus.pad1.left = pressed,
                        Button::DPadRight => self.bus.pad1.right = pressed,
                        Button::Select => self.bus.pad1.select = pressed,
                        Button::Start => self.bus.pad1.start = pressed,
                        Button::LeftTrigger | Button::RightTrigger => {
                            self.bus.pad1.a = pressed;
                        }
                        Button::LeftTrigger2 | Button::RightTrigger2 => {
                            self.bus.pad1.b = pressed;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if let Some(window) = &self.window {
            window.request_redraw();
        }

        event_loop.set_control_flow(ControlFlow::Poll);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let rom_path = if args.len() > 1 {
        Some(args[1].clone())
    } else {
        None
    };

    if rom_path.is_none() {
        eprintln!("No ROM path provided, downloading default ROM...");
        eprintln!("  {}", DEFAULT_ROM);
        eprintln!(
            "Usage: {} <rom.nes>  (or run without args to download)",
            args[0]
        );
    }

    let mut app = App::new(rom_path);
    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).expect("event loop failed");
}
