use std::time::Duration;

use super::{
    bindings::{AxisBind, Bindings, InputBind},
    raw::RawInputManagerState,
};

#[derive(Debug)]
pub struct InputManagerState<B: InputBind> {
    /// The mouse sensitivity in the x and y direction. Use a negative value to reverse the mouse.
    pub mouse_sensitivity: [f64; 2],
    /// Input bindings
    pub bindings: Bindings<B>,
    /// The current time elapsed since the start of the event loop scaled by the `time_scale`.
    pub time: Duration,
    /// The time scale controls how fast time runs. A value of 1.0 is normal. A value of < 1.0 is slower than normal and > 1.0 is faster than normal.
    pub time_scale: f32,
    /// The ema alpha used to smooth the frame rate that is returned by [`Self::smooth_frame_rate`]. Defaults to 0.05
    pub smooth_frame_rate_alpha: f32,
    /// The ema smoothed frame rate
    pub smooth_frame_rate: f32,
    pub raw: RawInputManagerState,
}

impl<B: InputBind> Default for InputManagerState<B> {
    fn default() -> Self {
        Self {
            mouse_sensitivity: [1.0, 1.0],
            bindings: Bindings::default(),
            time: Duration::default(),
            time_scale: 1.0,
            smooth_frame_rate_alpha: 0.05,
            smooth_frame_rate: 0.0,
            raw: RawInputManagerState::default(),
        }
    }
}

impl<B: InputBind> InputManagerState<B> {
    pub fn preupdate(&mut self) {
        self.raw.preupdate();
        self.time += self.delta_time();
        self.smooth_frame_rate = self.smooth_frame_rate_alpha * self.raw.frame_rate()
            + (1.0 - self.smooth_frame_rate_alpha) * self.smooth_frame_rate;
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
            m[0] * self.mouse_sensitivity[0],
            m[1] * self.mouse_sensitivity[1],
        )
    }

    /// Returns the time between the last update and the update before it taking into account the `time_scale`.
    pub fn delta_time(&self) -> Duration {
        self.raw.delta_time().mul_f32(self.time_scale)
    }

    /// Returns the time between the last update and the update before it taking into account the `time_scale` as an f32.
    ///
    /// Equivalent to [`Self::delta_time`] followed by [`Duration::as_secs_f32`]
    pub fn delta_time_f32(&self) -> f32 {
        self.delta_time().as_secs_f32()
    }

    /// Returns the time between the last update and the update before it taking into account the `time_scale` as an f64.
    ///
    /// Equivalent to [`Self::delta_time`] followed by [`Duration::as_secs_f64`]
    pub fn delta_time_f64(&self) -> f64 {
        self.delta_time().as_secs_f64()
    }

    /// Get the 1-D axis
    #[expect(clippy::needless_pass_by_value)]
    pub fn axis(&self, bind: AxisBind<B>) -> f32 {
        (if self.held(bind.pos) { 1.0 } else { 0.0 })
            - (if self.held(bind.neg) { 1.0 } else { 0.0 })
    }

    /// Get the N-D axis
    pub fn axis_n<const N: usize>(&self, binds: [AxisBind<B>; N]) -> [f32; N] {
        binds.map(|axis| self.axis(axis))
    }

    /// Get the N-D axis with the length of 1 or 0
    pub fn axis_n_norm<const N: usize>(&self, binds: [AxisBind<B>; N]) -> [f32; N] {
        let axes = self.axis_n(binds);
        let sqr_mag: f32 = axes.iter().map(|x| x * x).sum();
        if sqr_mag == 0.0 {
            [0.0; N]
        } else {
            let m = sqr_mag.sqrt();
            axes.map(|x| x / m)
        }
    }

    /// Returns [`true`] every [`time`] interval measured in seconds
    #[cfg(feature = "unstable")]
    pub fn every(&self, time: f32) -> bool {
        // TODO: The soundness of the floating point number maths here needs to be verified that every is called every time
        self.time.as_secs_f32() % time < self.delta_time_f32()
    }
}
