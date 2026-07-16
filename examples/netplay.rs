/// NES Netplay Example - 2-player over UDP
///
/// Frame-locked lockstep: both sides exchange inputs for frame N,
/// THEN advance. This guarantees both consoles see identical game state.
///
/// Usage (Player 1 - host):
///   cargo run --example netplay --release -- <rom.nes> --listen 0.0.0.0:9400
///
/// Usage (Player 2 - join):
///   cargo run --example netplay --release -- <rom.nes> --connect <host-ip>:9400
use std::net::UdpSocket;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU8, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use nes::bus::Bus;
use nes::cpu::CpuRp2a03;
use nes::rom::Rom;
use nes::{reset, tick};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::HeapRb;
use ringbuf::traits::{Consumer, Producer, Split};
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes};

use gilrs::{Button, Gilrs};

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

fn scale_frame(src: &[u8; (NES_W * NES_H) as usize], dst: &mut [u32], dw: u32, dh: u32) {
    for y in 0..dh {
        for x in 0..dw {
            let sx = x * NES_W / dw;
            let sy = y * NES_H / dh;
            dst[(y * dw + x) as usize] = nes_colour(src[(sy * NES_W + sx) as usize]);
        }
    }
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

/// Non-blocking netplay: continuously exchanges inputs in a background thread.
/// Main thread reads the latest remote input without ever blocking.
struct NetPeer {
    /// Shared atomic: main thread writes local input, network thread reads it.
    local_input: std::sync::Arc<AtomicU8>,
    /// Shared atomic: network thread writes remote input, main thread reads it.
    remote_input: std::sync::Arc<AtomicU8>,
}

impl NetPeer {
    fn new(remote_addr: &str, listen_addr: &str) -> Self {
        // Use ephemeral port (port 0) to avoid "address in use" errors
        let sock = if listen_addr.ends_with(":0") || listen_addr == "0.0.0.0:0" {
            UdpSocket::bind("0.0.0.0:0").expect("bind failed")
        } else {
            UdpSocket::bind(listen_addr).expect("bind failed")
        };
        let _ = sock.set_read_timeout(Some(Duration::from_millis(16)));
        let _ = sock.connect(remote_addr);

        let local = std::sync::Arc::new(AtomicU8::new(0));
        let remote = std::sync::Arc::new(AtomicU8::new(0));
        let local_t = local.clone();
        let remote_t = remote.clone();
        let sock2 = sock.try_clone().expect("clone failed");

        thread::spawn(move || {
            let mut send_buf = [0u8; 1];
            let mut recv_buf = [0u8; 1];
            let mut last_sent: u8 = 0;
            loop {
                let input = local_t.load(Ordering::Relaxed);
                if input != last_sent {
                    send_buf[0] = input;
                    let _ = sock2.send(&send_buf);
                    last_sent = input;
                }
                if let Ok(n) = sock2.recv(&mut recv_buf) {
                    if n == 1 {
                        remote_t.store(recv_buf[0], Ordering::Relaxed);
                    }
                }
                thread::sleep(Duration::from_millis(1));
            }
        });

        Self {
            local_input: local,
            remote_input: remote,
        }
    }

    /// Send our local input (non-blocking atomic write)
    fn send_input(&self, local: u8) {
        self.local_input.store(local, Ordering::Relaxed);
    }

    /// Read opponent's latest input (non-blocking atomic read)
    fn recv_input(&self) -> u8 {
        self.remote_input.load(Ordering::Relaxed)
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
        ringbuf::CachingProd<std::sync::Arc<ringbuf::SharedRb<ringbuf::storage::Heap<f32>>>>,
    >,
    net: Option<NetPeer>,
    is_player1: bool,
}

impl App {
    fn new(
        rom_path: &str,
        connect_to: String,
        listen_on: Option<String>,
        is_player1: bool,
    ) -> Self {
        let data =
            std::fs::read(rom_path).unwrap_or_else(|_| panic!("failed to read {}", rom_path));
        let rom = Rom::new(&data).expect("invalid iNES ROM");

        let mut cpu = CpuRp2a03::new(0);
        let mut bus = Bus::new(rom.create_mapper());
        reset(&mut cpu, &mut bus);

        let net = Some(NetPeer::new(
            &connect_to,
            &listen_on.unwrap_or_else(|| "0.0.0.0:0".to_string()),
        ));

        Self {
            cpu,
            bus,
            gilrs: Gilrs::new().expect("gilrs init failed"),
            window: None,
            ctx: None,
            surface: None,
            frame_timer: Instant::now(),
            frame_dur: Duration::from_nanos(1_000_000_000 / 60),
            acc: Duration::new(0, 0),
            audio_stream: None,
            audio_tx: None,
            net,
            is_player1,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let w = (NES_W as f64 * 8.0 / 7.0 * SCALE as f64).round() as u32;
        let h = NES_H * SCALE;

        let window = event_loop
            .create_window(
                WindowAttributes::default()
                    .with_title("x-nes netplay")
                    .with_inner_size(winit::dpi::LogicalSize::new(w, h)),
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

        // Init audio
        if self.audio_stream.is_none() {
            let host = cpal::default_host();
            if let Some(device) = host.default_output_device() {
                if let Ok(supported) = device.default_output_config() {
                    let sample_rate = supported.sample_rate();
                    let channels = supported.channels();
                    self.bus.apu.set_sample_rate(sample_rate as f64);

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
                // Host (P1): keyboard → pad1. Joiner (P2): keyboard → pad2.
                let pad = if self.is_player1 {
                    &mut self.bus.pad1
                } else {
                    &mut self.bus.pad2
                };
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyZ) => pad.b = pressed,
                    PhysicalKey::Code(KeyCode::KeyX) => pad.a = pressed,
                    PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => {
                        pad.select = pressed
                    }
                    PhysicalKey::Code(KeyCode::Enter) => pad.start = pressed,
                    PhysicalKey::Code(KeyCode::ArrowUp) => pad.up = pressed,
                    PhysicalKey::Code(KeyCode::ArrowDown) => pad.down = pressed,
                    PhysicalKey::Code(KeyCode::ArrowLeft) => pad.left = pressed,
                    PhysicalKey::Code(KeyCode::ArrowRight) => pad.right = pressed,
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

        while self.acc >= self.frame_dur {
            // Non-blocking netplay exchange
            // Host (P1): sends own pad1, receives opponent's input as pad2
            // Joiner (P2): sends own pad2, receives host's input as pad1
            if let Some(net) = &self.net {
                let (local_src, remote_dst) = if self.is_player1 {
                    (&self.bus.pad1, &mut self.bus.pad2)
                } else {
                    (&self.bus.pad2, &mut self.bus.pad1)
                };
                let local_byte = pad_to_byte(local_src);
                net.send_input(local_byte);
                let remote_byte = net.recv_input();
                byte_to_pad(remote_byte, remote_dst);
            }

            // Both sides now have the same inputs for this frame.
            // Advance exactly one frame (deterministic).
            while !self.bus.ppu.frame_complete {
                tick(&mut self.cpu, &mut self.bus);
            }
            self.bus.ppu.frame_complete = false;
            self.acc -= self.frame_dur;

            // Push audio
            if let Some(tx) = &mut self.audio_tx {
                let n = self.bus.apu.sample_count;
                if n > 0 {
                    let _ = tx.push_slice(&self.bus.apu.audio_samples[..n]);
                }
                self.bus.apu.sample_count = 0;
            }
        }

        // Poll controller
        while let Some(gilrs::Event { event, .. }) = self.gilrs.next_event() {
            if let gilrs::EventType::ButtonChanged(button, val, _) = event {
                let pressed = val > 0.5;
                let pad = if self.is_player1 {
                    &mut self.bus.pad1
                } else {
                    &mut self.bus.pad2
                };
                match button {
                    Button::South => pad.a = pressed,
                    Button::East => pad.b = pressed,
                    Button::DPadUp => pad.up = pressed,
                    Button::DPadDown => pad.down = pressed,
                    Button::DPadLeft => pad.left = pressed,
                    Button::DPadRight => pad.right = pressed,
                    Button::Select => pad.select = pressed,
                    Button::Start => pad.start = pressed,
                    Button::LeftTrigger | Button::RightTrigger => pad.a = pressed,
                    Button::LeftTrigger2 | Button::RightTrigger2 => pad.b = pressed,
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
    if args.len() < 3 {
        eprintln!("Usage:");
        eprintln!("  Host: {} <rom.nes> --listen 0.0.0.0:9400", args[0]);
        eprintln!("  Join: {} <rom.nes> --connect <host-ip>:9400", args[0]);
        eprintln!();
        eprintln!("Example:");
        eprintln!("  P1: {} contra.nes --listen 0.0.0.0:9400", args[0]);
        eprintln!("  P2: {} contra.nes --connect 127.0.0.1:9400", args[0]);
        return;
    }

    let mut rom_path = None;
    let mut connect_to = None;
    let mut listen_on = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--connect" => {
                i += 1;
                connect_to = Some(args[i].clone());
            }
            "--listen" => {
                i += 1;
                listen_on = Some(args[i].clone());
            }
            s if !s.starts_with("--") => rom_path = Some(s.to_string()),
            _ => {
                eprintln!("Unknown: {}", args[i]);
                return;
            }
        }
        i += 1;
    }

    let rom = rom_path.expect("no ROM specified");

    match (connect_to, listen_on) {
        // Joiner: connected to host → I'm Player 2
        (Some(remote), listen) => {
            let mut app = App::new(&rom, remote, listen, false);
            let el = EventLoop::new().expect("event loop");
            el.set_control_flow(ControlFlow::Poll);
            el.run_app(&mut app).expect("event loop failed");
        }
        // Host: listening for P2 → I'm Player 1
        (None, Some(local)) => {
            eprintln!("Waiting for player 2 on {} ...", local);
            let sock = UdpSocket::bind(&local).expect("bind failed");
            sock.set_read_timeout(Some(Duration::from_secs(300))).ok();
            let mut buf = [0u8; 1];
            let (_, src) = sock.recv_from(&mut buf).expect("no connection");
            eprintln!("Player 2 connected from {}", src);
            let remote = format!("{}:{}", src.ip(), src.port());
            let mut app = App::new(&rom, remote, Some(local), true);
            let el = EventLoop::new().expect("event loop");
            el.set_control_flow(ControlFlow::Poll);
            el.run_app(&mut app).expect("event loop failed");
        }
        _ => {
            eprintln!("Use --listen (host) or --connect (join)");
        }
    }
}
