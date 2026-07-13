use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use nes::bus::Bus;
use nes::cpu::Cpu6502;
use nes::rom::Rom;
use nes::{reset, tick};

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::traits::{Producer, Consumer, Split};
use ringbuf::HeapRb;

const NES_W: u32 = 256;
const NES_H: u32 = 240;
const NES_PAR: f64 = 8.0 / 7.0;
const SCALE: u32 = 3;
const DEFAULT_ROM: &str =
    "https://github.com/NovaSquirrel/NovaTheSquirrel/releases/download/v1.0.6a/nova.nes";

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
    cpu: Cpu6502,
    bus: Bus<'static>,
    window: Option<std::rc::Rc<Window>>,
    ctx: Option<Context<std::rc::Rc<Window>>>,
    surface: Option<Surface<std::rc::Rc<Window>, std::rc::Rc<Window>>>,
    frame_timer: Instant,
    frame_dur: Duration,
    acc: Duration,
    audio_stream: Option<cpal::Stream>,
    audio_tx: Option<ringbuf::CachingProd<std::sync::Arc<ringbuf::SharedRb<ringbuf::storage::Heap<f32>>>>>,
}

impl App {
    fn new(rom_path: Option<String>) -> Self {
        let path = rom_path.as_deref().unwrap_or(DEFAULT_ROM);
        let data = load_rom(path);
        let rom = Rom::new(&data).expect("invalid iNES ROM");

        let prg: &'static [u8] = Box::leak(rom.prg.to_vec().into_boxed_slice());
        let chr: &'static [u8] = Box::leak(rom.chr.to_vec().into_boxed_slice());

        let mut cpu = Cpu6502::new(0);
        let mut bus = Bus::new(prg, chr, rom.mirroring);
        reset(&mut cpu, &mut bus);

        Self {
            cpu,
            bus,
            window: None,
            ctx: None,
            surface: None,
            frame_timer: Instant::now(),
            frame_dur: Duration::from_nanos(1_000_000_000 / 60),
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
                        let cpu_freq: f64 = 1_789_773.0;
                        self.bus.apu.set_sample_rate(sample_rate as f64);
                        
                        let config: cpal::StreamConfig = supported.into();
                        let rb = HeapRb::<f32>::new(16384);
                        let (prod, mut cons) = rb.split();
                        let ch = channels as usize;
                        
                        let err_fn = |err| eprintln!("audio stream error: {}", err);
                        
                        let stream = device.build_output_stream(
                            config,
                            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                                // Fill all channels with the same mono sample
                                for frame in data.chunks_mut(ch) {
                                    let s = cons.try_pop().unwrap_or(0.0);
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

            if let Some(tx) = &mut self.audio_tx {
                let n = self.bus.apu.sample_count;
                if n > 0 {
                    let _pushed = tx.push_slice(&self.bus.apu.audio_samples[..n]);
                }
                self.bus.apu.sample_count = 0;
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
