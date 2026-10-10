//! Spike: apakah popup X11 biasa (managed) mendapat fokus keyboard di GNOME Wayland?
//! Pemakaian: focus [--delay-ms N] [--secs N]
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::platform::x11::{EventLoopBuilderExtX11, WindowAttributesExtX11, WindowType};
use winit::window::{Window, WindowId, WindowLevel};

struct App {
    start: Instant,
    delay: Duration,
    secs: Duration,
    popup: Option<Window>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _el: &ActiveEventLoop) {}

    fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, ev: WindowEvent) {
        let t = self.start.elapsed().as_millis();
        match ev {
            WindowEvent::Focused(f) => eprintln!("[{t} ms] popup Focused({f})"),
            WindowEvent::KeyboardInput { event, is_synthetic, .. } => eprintln!("[{t} ms] key {:?} state={:?} repeat={} synthetic={is_synthetic}", event.logical_key, event.state, event.repeat),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        let now = self.start.elapsed();
        if self.popup.is_none() && now >= self.delay {
            let attrs = Window::default_attributes()
                .with_title("pointer-text")
                .with_decorations(false)
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_inner_size(LogicalSize::new(420.0, 64.0))
                .with_position(LogicalPosition::new(300.0, 300.0))
                .with_active(true)
                .with_x11_window_type(vec![WindowType::Dialog]);
            let w = el.create_window(attrs).expect("popup");
            w.focus_window();
            eprintln!("[{} ms] popup dibuat", now.as_millis());
            self.popup = Some(w);
        }
        if now >= self.secs {
            el.exit();
        } else {
            el.set_control_flow(ControlFlow::WaitUntil(self.start + now.max(self.delay).min(self.secs) + Duration::from_millis(50)));
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let num = |flag: &str, d: u64| {
        args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(d)
    };
    let el = EventLoop::builder().with_x11().build().unwrap();
    let mut app = App {
        start: Instant::now(),
        delay: Duration::from_millis(num("--delay-ms", 0)),
        secs: Duration::from_secs(num("--secs", 3)),
        popup: None,
    };
    el.run_app(&mut app).unwrap();
}
