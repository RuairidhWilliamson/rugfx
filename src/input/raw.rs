use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, ElementState, MouseScrollDelta, StartCause, WindowEvent},
};

use super::Input;

/// Stores state about keys, mouse motion, timing and other window events.
pub struct RawInputManager<H> {
    pub handler: H,
    state: RawInputManagerState,
}

#[derive(Debug)]
pub struct RawInputManagerState {
    keys_held: HashSet<Input>,
    keys_pressed: HashSet<Input>,
    keys_released: HashSet<Input>,

    mouse_motion: [f64; 2],
    mouse_position: [f64; 2],
    mouse_wheel_delta: [f32; 2],

    start: Instant,
    last_update: Instant,
    update_delta: Duration,

    resize: Option<PhysicalSize<u32>>,
    close_requested: bool,
    loop_exiting: bool,
}

impl<H: RawInputHandler> ApplicationHandler for RawInputManager<H> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.handler.resumed(event_loop)
    }

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.state.process_window_event(event);
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        self.state.preupdate();
        self.handler.update(event_loop, &self.state);
        // We can't draw on the StartCause::Init new_events because resume has not been called and hence created the window
        if cause != StartCause::Init {
            self.handler.draw(event_loop, &self.state);
        }
        self.state.clear();
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.state.mouse_motion[0] += delta.0;
            self.state.mouse_motion[1] += delta.1;
        }
    }

    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.state.loop_exiting = true;
    }
}

pub trait RawInputHandler {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop);
    fn update(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        input: &RawInputManagerState,
    );
    fn draw(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        input: &RawInputManagerState,
    );
}

impl<H: RawInputHandler> RawInputManager<H> {
    pub fn new(handler: H) -> Self {
        Self {
            handler,
            state: RawInputManagerState::default(),
        }
    }
}

impl Default for RawInputManagerState {
    fn default() -> Self {
        Self {
            keys_held: HashSet::default(),
            keys_pressed: HashSet::default(),
            keys_released: HashSet::default(),
            mouse_motion: [0.0, 0.0],
            mouse_position: [0.0, 0.0],
            mouse_wheel_delta: [0.0, 0.0],

            start: Instant::now(),
            last_update: Instant::now(),
            update_delta: Duration::default(),

            resize: None,
            close_requested: false,
            loop_exiting: false,
        }
    }
}

impl RawInputManagerState {
    pub fn process_window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                self.update_input(event.physical_key.into(), event.state);
            }
            WindowEvent::CloseRequested => {
                self.close_requested = true;
            }
            WindowEvent::Resized(size) => {
                self.resize = Some(size);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = [position.x, position.y];
            }
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(x, y),
                ..
            } => {
                self.mouse_wheel_delta[0] += x;
                self.mouse_wheel_delta[1] += y;
            }
            WindowEvent::MouseInput { button, state, .. } => {
                self.update_input(button.into(), state);
            }
            WindowEvent::Focused(false) => {
                // When lost focus clear the keys held
                self.keys_held.clear();
            }
            _ => (),
        }
    }

    fn update_input(&mut self, input: Input, state: ElementState) {
        match state {
            ElementState::Pressed => {
                self.keys_held.insert(input);
                self.keys_pressed.insert(input);
            }
            ElementState::Released => {
                self.keys_held.remove(&input);
                self.keys_released.insert(input);
            }
        }
    }

    pub fn preupdate(&mut self) {
        let now = Instant::now();
        self.update_delta = now.saturating_duration_since(self.last_update);
        self.last_update = now;
    }

    pub fn clear(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_motion = [0.0; 2];
        self.mouse_wheel_delta = [0.0; 2];
        self.resize = None;
        self.close_requested = false;
    }

    /// If a key was pressed since the last update
    pub fn pressed(&self, input: &Input) -> bool {
        self.keys_pressed.contains(input)
    }

    /// If a key was held at all since the last update
    pub fn held(&self, input: &Input) -> bool {
        self.keys_held.contains(input)
    }

    /// If a key was released since the last update
    pub fn released(&self, input: &Input) -> bool {
        self.keys_released.contains(input)
    }

    /// The motion of the mouse since the last update
    pub fn mouse_motion(&self) -> [f64; 2] {
        self.mouse_motion
    }

    /// Returns the mouse position relative to the current window
    pub fn mouse_position(&self) -> [f64; 2] {
        self.mouse_position
    }

    /// The time elapsed between the last update and the previous
    pub fn delta_time(&self) -> Duration {
        self.update_delta
    }

    /// The time elapsed between the last update and the previous as a f32
    pub fn delta_time_f32(&self) -> f32 {
        self.update_delta.as_secs_f32()
    }

    /// The time elapsed between the last update and the previous as a f64
    pub fn delta_time_f64(&self) -> f64 {
        self.update_delta.as_secs_f64()
    }

    /// The current framerate based on the [`Self::delta_time`]
    pub fn frame_rate(&self) -> f32 {
        1.0 / self.delta_time_f32()
    }

    /// Returns Some if the window was resized
    ///
    /// See [`winit::event::WindowEvent::Resized`]
    pub fn resized(&self) -> &Option<PhysicalSize<u32>> {
        &self.resize
    }

    /// Returns true if the os/window manager has requested the window close, normally by clicking the close button
    ///
    /// See [`winit::event::WindowEvent::CloseRequested`]
    pub fn close_requested(&self) -> bool {
        self.close_requested
    }

    /// Returns true if the winit event loop was destroyed.
    ///
    /// See [`winit::event::Event::LoopExiting`].
    pub fn loop_exiting(&self) -> bool {
        self.loop_exiting
    }

    /// The total time since the start of the game
    pub fn game_time(&self) -> Duration {
        self.last_update.saturating_duration_since(self.start)
    }

    /// Runs every duration
    #[cfg(feature = "unstable")]
    pub fn every(&self, duration: Duration) -> bool {
        let game_time = self.game_time();
        game_time.as_secs_f64() % duration.as_secs_f64() < self.update_delta.as_secs_f64()
    }
}
