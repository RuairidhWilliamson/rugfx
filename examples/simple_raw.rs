#![allow(clippy::unwrap_used)]

use std::{num::NonZeroU32, rc::Rc};

use rugfx::input::{
    raw::{RawInputHandler, RawInputManager, RawInputManagerState},
    Input,
};
use softbuffer::Surface;
use winit::{
    keyboard::KeyCode,
    window::{Window, WindowAttributes},
};

#[derive(Default)]
struct App {
    window: Option<CreatedWindow>,
}

struct CreatedWindow {
    window: Rc<Window>,
    surface: Surface<Rc<Window>, Rc<Window>>,
}

impl CreatedWindow {
    fn new(event_loop: &winit::event_loop::ActiveEventLoop) -> Self {
        let window = Rc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );
        let ctx = softbuffer::Context::new(window.clone()).unwrap();
        let surface = softbuffer::Surface::new(&ctx, window.clone()).unwrap();
        Self { window, surface }
    }

    fn draw_empty_window(&mut self) {
        // Draw empty window so that events work
        let size = self.window.inner_size();
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return;
        };

        self.surface
            .resize(width, height)
            .expect("Failed to resize the softbuffer surface");

        // Fill a buffer with a solid color.
        const DARK_GRAY: u32 = 0xFF181818;
        let mut buffer = self
            .surface
            .buffer_mut()
            .expect("Failed to get the softbuffer buffer");
        buffer.fill(DARK_GRAY);
        buffer
            .present()
            .expect("Failed to present the softbuffer buffer");
    }
}

impl RawInputHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window = Some(CreatedWindow::new(event_loop));
    }

    fn update(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        input: &RawInputManagerState,
    ) {
        if input.pressed(&Input::Key(winit::keyboard::PhysicalKey::Code(
            KeyCode::KeyA,
        ))) {
            println!("A pressed");
        }
        if input.close_requested() {
            event_loop.exit();
        }
    }

    fn draw(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _input: &RawInputManagerState,
    ) {
        self.window.as_mut().unwrap().draw_empty_window();
    }
}

fn main() {
    let app = App::default();
    let mut input_manager = RawInputManager::new(app);
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    event_loop.run_app(&mut input_manager).unwrap();
}
