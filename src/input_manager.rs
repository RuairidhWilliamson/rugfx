use std::time::Duration;

use winit::event::Event;

use crate::{
    bindings::{AxisBind, Bindings, InputBind},
    raw::RawInputManager,
};

/// Input manager manages takes events and stores them in a easy to use interface.
///
/// Input manager is the recommended interface to use. Pass it all your [`winit::event::Event`]s from your event loop and it will manage state and provide a clean api to access it.
#[derive(Debug)]
pub struct InputManager<B: InputBind> {
    /// The mouse sensitivity in the x and y direction. Use a negative value to reverse the mouse.
    pub mouse_sensitivity: (f64, f64),
    /// Input bindings
    pub bindings: Bindings<B>,
    /// The current time elapsed since the start of the event loop scaled by the time_scale.
    pub time: Duration,
    /// The time scale controls how fast time runs. A value of 1.0 is normal. A value of < 1.0 is slower than normal and > 1.0 is faster than normal.
    pub time_scale: f32,
    /// The ema alpha used to smooth the frame rate that is returned by [`Self::smooth_frame_rate`]. Defaults to 0.05
    pub smooth_frame_rate_alpha: f32,
    /// The ema smoothed frame rate
    pub smooth_frame_rate: f32,
    /// The underlying raw input manager
    pub raw: RawInputManager,
}

impl<B: InputBind> Default for InputManager<B> {
    fn default() -> Self {
        Self {
            mouse_sensitivity: (1.0, 1.0),
            bindings: Bindings::default(),
            time: Duration::default(),
            time_scale: 1.0,
            smooth_frame_rate_alpha: 0.05,
            smooth_frame_rate: 0.0,
            raw: RawInputManager::default(),
        }
    }
}

impl<B: InputBind> InputManager<B> {
    /// Pass the [`winit::event::Event`] to the input manager. Should be called every time in the event loop.
    pub fn pass_event<T: std::fmt::Debug>(&mut self, event: &Event<T>) -> bool {
        let is_update = self.raw.pass_event(event);
        if is_update {
            self.time += self.delta_time();
            self.smooth_frame_rate = self.smooth_frame_rate_alpha * self.raw.frame_rate()
                + (1.0 - self.smooth_frame_rate_alpha) * self.smooth_frame_rate;
        }
        is_update
    }

    /// Returns true if the binding was pressed since the last update
    pub fn pressed(&self, input: &B) -> bool {
        self.bindings
            .transform(input)
            .iter()
            .any(|k| self.raw.pressed(k))
    }

    /// Returns true if the binding was held at any point since the last update
    pub fn held(&self, input: &B) -> bool {
        self.bindings
            .transform(input)
            .iter()
            .any(|k| self.raw.held(k))
    }

    /// Returns true if the binding as released since the last update
    pub fn released(&self, input: &B) -> bool {
        self.bindings
            .transform(input)
            .iter()
            .any(|k| self.raw.released(k))
    }

    /// The mouse motion since the last update multiplied by the mouse sensitivity
    pub fn mouse_motion(&self) -> (f64, f64) {
        let m = self.raw.mouse_motion();
        (
            m.0 * self.mouse_sensitivity.0,
            m.1 * self.mouse_sensitivity.1,
        )
    }

    /// Returns the time between the last update and the update before it taking into account the time_scale.
    pub fn delta_time(&self) -> Duration {
        self.raw.delta_time().mul_f32(self.time_scale)
    }

    /// Returns the time between the last update and the update before it taking into account the time_scale as an f32.
    ///
    /// Equivalent to [`Self::delta_time`] followed by [`Duration::as_secs_f32`]
    pub fn delta_time_f32(&self) -> f32 {
        self.delta_time().as_secs_f32()
    }

    /// Returns the time between the last update and the update before it taking into account the time_scale as an f64.
    ///
    /// Equivalent to [`Self::delta_time`] followed by [`Duration::as_secs_f64`]
    pub fn delta_time_f64(&self) -> f64 {
        self.delta_time().as_secs_f64()
    }

    /// Get the 1D axis
    pub fn axis(&self, bind: AxisBind<B>) -> f32 {
        (if self.held(bind.pos) { 1.0 } else { 0.0 })
            + (if self.held(bind.neg) { 1.0 } else { 0.0 })
    }

    /// Get the 2D axis
    pub fn axis2(&self, binds: [AxisBind<B>; 2]) -> [f32; 2] {
        let [x, y] = binds;
        [self.axis(x), self.axis(y)]
    }

    /// Get the 2D axis on the unit circle
    pub fn axis2_norm(&self, binds: [AxisBind<B>; 2]) -> [f32; 2] {
        let [x, y] = self.axis2(binds);
        let m2 = x * x + y * y;
        if m2 == 0.0 {
            [0.0, 0.0]
        } else {
            let m = m2.sqrt();
            [x / m, y / m]
        }
    }

    /// Get the 3D axis
    pub fn axis3(&self, binds: [AxisBind<B>; 3]) -> [f32; 3] {
        let [x, y, z] = binds;
        [self.axis(x), self.axis(y), self.axis(z)]
    }

    /// Get the 3D axis on the unit sphere
    pub fn axis3_norm(&self, binds: [AxisBind<B>; 3]) -> [f32; 3] {
        let [x, y, z] = self.axis3(binds);
        let m2 = x * x + y * y + z * z;
        if m2 == 0.0 {
            [0.0, 0.0, 0.0]
        } else {
            let m = m2.sqrt();
            [x / m, y / m, z / m]
        }
    }

    /// Returns [`true`] every [`time`] interval
    #[cfg(feature = "unstable")]
    pub fn every(&self, time: f32) -> bool {
        // TODO: The soundness of the floating point number maths here needs to be verified that every is called every time
        self.time.as_secs_f32() % time < self.delta_time_f32()
    }
}

#[cfg(test)]
mod tests {
    use winit::event::VirtualKeyCode;

    use super::InputManager;

    #[test]
    fn no_binds() {
        let im = InputManager::<()>::default();
        assert!(!im.raw.pressed(&VirtualKeyCode::Escape.into()));
    }
}
