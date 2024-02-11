#![allow(clippy::unwrap_used)]

use std::{num::NonZeroU32, rc::Rc};

use rugfx::{
    dry_binds,
    input::{Bindings, InputManager},
};
use winit::keyboard::KeyCode;

#[derive(Debug, PartialEq, Eq, Hash)]
enum Bind {
    A,
    B,
    C,
    Exit,
}

fn main() {
    let bindings = dry_binds! {
        KeyCode::KeyA => Bind::A,
        KeyCode::KeyB => Bind::B,
        KeyCode::KeyC => Bind::C,
        KeyCode::Escape => Bind::Exit,
    };
    let mut input_manager = InputManager {
        bindings,
        ..InputManager::<Bind>::default()
    };
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window = Rc::new(winit::window::Window::new(&event_loop).unwrap());
    let ctx = softbuffer::Context::new(window.clone()).unwrap();
    let mut surface = softbuffer::Surface::new(&ctx, window.clone()).unwrap();
    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(winit::event_loop::ControlFlow::Wait);
            if input_manager.pass_event(&event) {
                if input_manager.pressed(&Bind::A) {
                    println!("A pressed");
                }
                if input_manager.pressed(&Bind::B) {
                    println!("B pressed");
                }
                if input_manager.pressed(&Bind::C) {
                    println!("C pressed");
                }

                if input_manager.pressed(&Bind::Exit) {
                    println!("Exit button pressed");
                    elwt.exit();
                }
                if input_manager.raw.close_requested() {
                    println!("Close requested");
                    elwt.exit();
                }

                // Draw empty window so that events work
                let size = window.inner_size();
                let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                else {
                    return;
                };

                surface
                    .resize(width, height)
                    .expect("Failed to resize the softbuffer surface");

                // Fill a buffer with a solid color.
                const DARK_GRAY: u32 = 0xFF181818;
                let mut buffer = surface
                    .buffer_mut()
                    .expect("Failed to get the softbuffer buffer");
                buffer.fill(DARK_GRAY);
                buffer
                    .present()
                    .expect("Failed to present the softbuffer buffer");
            }
        })
        .unwrap();
}
