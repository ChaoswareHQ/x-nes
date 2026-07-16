/// Dual NES - local 2-player test with split-screen
///
/// Two emulators side-by-side in one window, connected via mpsc channels
/// (no network dependency - instant start, zero config).
///
/// Controls:
///   Player 1 (left):  Z=B, X=A, Shift=Select, Enter=Start, Arrow keys=D-Pad
///   Player 2 (right): N=B, M=A, Comma=Select, Period=Start, WASD=D-Pad
use std::num::NonZeroU32;
use std::sync::mpsc;
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
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes};

const NES_W: u32 = 256;
const NES_H: u32 = 240;
const SCALE: u32 = 3;

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

fn pad_to_byte(pad: &nes::controller::Gamepad) -> u8 {
    let mut b = 0u8;
    if pad.a {
        b |= 0x01;
    }
    if pad.b {
        b |= 0x02;
    }
    if pad.select {
        b |= 0x04;
    }
    if pad.start {
        b |= 0x08;
    }
    if pad.up {
        b |= 0x10;
    }
    if pad.down {
        b |= 0x20;
    }
    if pad.left {
        b |= 0x40;
    }
    if pad.right {
        b |= 0x80;
    }
    b
}

fn byte_to_pad(b: u8, pad: &mut nes::controller::Gamepad) {
    pad.a = b & 0x01 != 0;
    pad.b = b & 0x02 != 0;
    pad.select = b & 0x04 != 0;
    pad.start = b & 0x08 != 0;
    pad.up = b & 0x10 != 0;
    pad.down = b & 0x20 != 0;
    pad.left = b & 0x40 != 0;
    pad.right = b & 0x80 != 0;
}

/// Channel-based peer (same-process, no network)
struct LocalPeer {
    local_tx: mpsc::Sender<u8>,
    remote_rx: mpsc::Receiver<u8>,
}

impl LocalPeer {
    /// Create a pair of connected peers.
    /// Returns (side_a, side_b) where side_a sends to side_b's rx and vice versa.
    fn pair() -> (Self, Self) {
        let (tx_a, rx_a) = mpsc::channel::<u8>();
        let (tx_b, rx_b) = mpsc::channel::<u8>();
        let a = Self {
            local_tx: tx_a,
            remote_rx: rx_b,
        };
        let b = Self {
            local_tx: tx_b,
            remote_rx: rx_a,
        };
        (a, b)
    }

    fn send_input(&self, local: u8) {
        let _ = self.local_tx.send(local);
    }

    fn recv_input(&self) -> u8 {
        let mut latest = 0u8;
        while let Ok(remote) = self.remote_rx.try_recv() {
            latest = remote;
        }
        latest
    }
}

struct EmuInstance {
    cpu: CpuRp2a03,
    bus: Bus,
    peer: Option<LocalPeer>,
}

impl EmuInstance {
    fn new(rom_path: &str) -> Self {
        let data = std::fs::read(rom_path).expect("failed to read ROM");
        let rom = Rom::new(&data).expect("invalid iNES ROM");
        let mut cpu = CpuRp2a03::new(0);
        let mut bus = Bus::new(rom.create_mapper());
        reset(&mut cpu, &mut bus);
        Self {
            cpu,
            bus,
            peer: None,
        }
    }

    fn tick_frame(&mut self) {
        while !self.bus.ppu.frame_complete {
            tick(&mut self.cpu, &mut self.bus);
        }
        self.bus.ppu.frame_complete = false;
    }

    fn sync_input(&mut self) {
        if let Some(peer) = &self.peer {
            let local = pad_to_byte(&self.bus.pad1);
            peer.send_input(local);
            let remote = peer.recv_input();
            byte_to_pad(remote, &mut self.bus.pad2);
        }
    }
}

struct App {
    emu1: EmuInstance,
    emu2: EmuInstance,
    window: Option<std::rc::Rc<Window>>,
    ctx: Option<Context<std::rc::Rc<Window>>>,
    surface: Option<Surface<std::rc::Rc<Window>, std::rc::Rc<Window>>>,
    frame_timer: Instant,
    frame_dur: Duration,
    acc: Duration,
    gilrs: Gilrs,
    audio_stream: Option<cpal::Stream>,
    audio_tx: Option<
        ringbuf::CachingProd<std::sync::Arc<ringbuf::SharedRb<ringbuf::storage::Heap<f32>>>>,
    >,
}

impl App {
    fn new(rom_path: &str) -> Self {
        let mut emu1 = EmuInstance::new(rom_path);
        let mut emu2 = EmuInstance::new(rom_path);

        // Connect them via channels (instant, no network)
        let (peer1, peer2) = LocalPeer::pair();
        emu1.peer = Some(peer1);
        emu2.peer = Some(peer2);

        Self {
            emu1,
            emu2,
            window: None,
            ctx: None,
            surface: None,
            frame_timer: Instant::now(),
            frame_dur: Duration::from_nanos(1_000_000_000 / 60),
            acc: Duration::new(0, 0),
            gilrs: Gilrs::new().expect("gilrs init failed"),
            audio_stream: None,
            audio_tx: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let w = NES_W * 2 * SCALE;
        let h = NES_H * SCALE;

        let window = event_loop
            .create_window(
                WindowAttributes::default()
                    .with_title("x-nes dual (2-player test)")
                    .with_inner_size(winit::dpi::LogicalSize::new(w as f64, h as f64)),
            )
            .expect("failed to create window");

        let rc = std::rc::Rc::new(window);
        let ctx = Context::new(rc.clone()).expect("context failed");
        let surface = Surface::new(&ctx, rc.clone()).expect("surface failed");

        self.window = Some(rc);
        self.ctx = Some(ctx);
        self.surface = Some(surface);
        self.frame_timer = Instant::now();
        event_loop.set_control_flow(ControlFlow::Poll);

        // Init audio (P1 only)
        if self.audio_stream.is_none() {
            let host = cpal::default_host();
            if let Some(device) = host.default_output_device() {
                if let Ok(supported) = device.default_output_config() {
                    let sample_rate = supported.sample_rate();
                    let channels = supported.channels();
                    self.emu1.bus.apu.set_sample_rate(sample_rate as f64);

                    let config: cpal::StreamConfig = supported.into();
                    let rb = HeapRb::<f32>::new(16384);
                    let (prod, mut cons) = rb.split();
                    let ch = channels as usize;

                    if let Ok(stream) = device.build_output_stream(
                        config,
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            for frame in data.chunks_mut(ch) {
                                let s = cons.try_pop().unwrap_or(0.0);
                                for sample in frame.iter_mut() {
                                    *sample = s;
                                }
                            }
                        },
                        |e| eprintln!("audio: {}", e),
                        None,
                    ) {
                        stream.play().ok();
                        self.audio_stream = Some(stream);
                        self.audio_tx = Some(prod);
                    }
                }
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
                    // P1 (left): Z, X, Shift, Enter, Arrows
                    PhysicalKey::Code(KeyCode::KeyZ) => self.emu1.bus.pad1.b = pressed,
                    PhysicalKey::Code(KeyCode::KeyX) => self.emu1.bus.pad1.a = pressed,
                    PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => {
                        self.emu1.bus.pad1.select = pressed
                    }
                    PhysicalKey::Code(KeyCode::Enter) => self.emu1.bus.pad1.start = pressed,
                    PhysicalKey::Code(KeyCode::ArrowUp) => self.emu1.bus.pad1.up = pressed,
                    PhysicalKey::Code(KeyCode::ArrowDown) => self.emu1.bus.pad1.down = pressed,
                    PhysicalKey::Code(KeyCode::ArrowLeft) => self.emu1.bus.pad1.left = pressed,
                    PhysicalKey::Code(KeyCode::ArrowRight) => self.emu1.bus.pad1.right = pressed,

                    // P2 (right): N, M, Comma, Period, WASD
                    PhysicalKey::Code(KeyCode::KeyN | KeyCode::KeyV) => {
                        self.emu2.bus.pad1.b = pressed
                    }
                    PhysicalKey::Code(KeyCode::KeyM) => self.emu2.bus.pad1.a = pressed,
                    PhysicalKey::Code(KeyCode::Comma) => self.emu2.bus.pad1.select = pressed,
                    PhysicalKey::Code(KeyCode::Period) => self.emu2.bus.pad1.start = pressed,
                    PhysicalKey::Code(KeyCode::KeyW) => self.emu2.bus.pad1.up = pressed,
                    PhysicalKey::Code(KeyCode::KeyS) => self.emu2.bus.pad1.down = pressed,
                    PhysicalKey::Code(KeyCode::KeyA) => self.emu2.bus.pad1.left = pressed,
                    PhysicalKey::Code(KeyCode::KeyD) => self.emu2.bus.pad1.right = pressed,
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(surface) = &mut self.surface {
                    let rc = self.window.as_ref().unwrap();
                    let size = rc.inner_size();
                    let dw = size.width.max(1);
                    let dh = size.height.max(1);
                    let half = dw / 2;
                    let mut buf = vec![0u32; (dw * dh) as usize];

                    // P1 (left half)
                    for y in 0..dh {
                        for x in 0..half {
                            let sx = x * NES_W / half;
                            let sy = y * NES_H / dh;
                            let c = self.emu1.bus.ppu.frame[(sy * NES_W + sx) as usize];
                            buf[(y * dw + x) as usize] = nes_colour(c);
                        }
                    }
                    // P2 (right half)
                    for y in 0..dh {
                        for x in half..dw {
                            let sx = (x - half) * NES_W / (dw - half);
                            let sy = y * NES_H / dh;
                            let c = self.emu2.bus.ppu.frame[(sy * NES_W + sx) as usize];
                            buf[(y * dw + x) as usize] = nes_colour(c);
                        }
                    }

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

        while self.acc >= self.frame_dur {
            // Exchange inputs between the two emulators
            self.emu1.sync_input();
            self.emu2.sync_input();

            // Tick both
            self.emu1.tick_frame();
            self.emu2.tick_frame();
            self.acc -= self.frame_dur;

            // Audio from P1
            if let Some(tx) = &mut self.audio_tx {
                let n = self.emu1.bus.apu.sample_count;
                if n > 0 {
                    let _ = tx.push_slice(&self.emu1.bus.apu.audio_samples[..n]);
                }
                self.emu1.bus.apu.sample_count = 0;
            }
        }

        // Poll game controllers (both map to P1's pad1 for now)
        while let Some(gilrs::Event { event, .. }) = self.gilrs.next_event() {
            if let gilrs::EventType::ButtonChanged(button, val, _) = event {
                let pressed = val > 0.5;
                match button {
                    Button::South => self.emu1.bus.pad1.a = pressed,
                    Button::East => self.emu1.bus.pad1.b = pressed,
                    Button::DPadUp => self.emu1.bus.pad1.up = pressed,
                    Button::DPadDown => self.emu1.bus.pad1.down = pressed,
                    Button::DPadLeft => self.emu1.bus.pad1.left = pressed,
                    Button::DPadRight => self.emu1.bus.pad1.right = pressed,
                    Button::Select => self.emu1.bus.pad1.select = pressed,
                    Button::Start => self.emu1.bus.pad1.start = pressed,
                    _ => {}
                }
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
    if args.len() < 2 {
        eprintln!("Usage: {} <rom.nes>", args[0]);
        return;
    }

    let mut app = App::new(&args[1]);
    let event_loop = EventLoop::new().expect("event loop failed");
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).expect("event loop failed");
}
