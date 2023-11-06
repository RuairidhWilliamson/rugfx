use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use winit::{
    dpi::PhysicalSize,
    event::{DeviceEvent, ElementState, Event, KeyboardInput, MouseScrollDelta, WindowEvent},
};

use crate::Input;

/// Stores state about keys, mouse motion, timing and other window events.
#[derive(Debug)]
pub struct RawInputManager {
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
    loop_destroyed: bool,
}

impl Default for RawInputManager {
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
            loop_destroyed: false,
        }
    }
}

impl RawInputManager {
    /// Pass all [`winit::event::Event`] in here
    pub fn pass_event<T: std::fmt::Debug>(&mut self, event: &Event<T>) -> bool {
        let is_update = matches!(event, Event::MainEventsCleared);
        match event {
            Event::NewEvents(_) => self.clear(),
            Event::WindowEvent { event, .. } => self.process_window_event(event),
            Event::DeviceEvent { event, .. } => self.process_device_event(event),
            Event::MainEventsCleared => self.process_events_cleared(),
            Event::LoopDestroyed => self.process_loop_destroyed(),
            _ => (),
        }
        is_update
    }

    fn process_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { input, .. } => self.process_keyboard_input(input),
            WindowEvent::CloseRequested => self.close_requested = true,
            WindowEvent::Resized(size) => self.resize = Some(*size),
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
                self.update_input((*button).into(), *state)
            }
            _ => (),
        }
    }

    fn process_device_event(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.mouse_motion[0] += delta.0;
            self.mouse_motion[1] += delta.1;
        }
    }

    fn process_events_cleared(&mut self) {
        let now = Instant::now();
        self.update_delta = now.saturating_duration_since(self.last_update);
        self.last_update = now;
    }

    fn process_keyboard_input(&mut self, input: &KeyboardInput) {
        self.update_input(input.scancode.into(), input.state);
        if let Some(vkey) = input.virtual_keycode {
            self.update_input(vkey.into(), input.state);
        };
    }

    fn process_loop_destroyed(&mut self) {
        self.loop_destroyed = true;
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

    fn clear(&mut self) {
        self.keys_pressed.clear();
        // self.keys_released.clear();
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
    /// See [`winit::event::Event::LoopDestroyed`].
    pub fn loop_destroyed(&self) -> bool {
        self.loop_destroyed
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
