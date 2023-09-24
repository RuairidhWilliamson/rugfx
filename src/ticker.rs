use std::time::{Duration, Instant};

/// Controls ticks by running every interval.
#[derive(Debug)]
pub struct Ticker {
    /// The tick interval, can be changed at any time and will update instantly
    pub interval: Duration,
    /// The number of ticks that have happened
    pub count: usize,
    last: Instant,
    is_tick: bool,
    /// Determines if ticks occur. Set to true to pause ticks, when set to false the next tick will most likely be instant.
    pub paused: bool,
}

impl Default for Ticker {
    fn default() -> Self {
        Self {
            interval: Duration::from_millis(250),
            count: 0,
            last: Instant::now(),
            is_tick: false,
            paused: false,
        }
    }
}

impl Ticker {
    /// Initialize a new ticker with an interval
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            ..Default::default()
        }
    }

    /// Call this every update
    pub fn update(&mut self) {
        let now = Instant::now();
        self.is_tick = !self.paused && now.saturating_duration_since(self.last) > self.interval;
        if self.is_tick {
            self.last = now;
            self.count += 1;
        }
    }

    /// Returns whether this update is a tick
    pub fn is_tick(&self) -> bool {
        self.is_tick
    }

    /// The saturated duration since the last tick
    pub fn time_since_last_tick(&self) -> Duration {
        Instant::now().saturating_duration_since(self.last)
    }

    /// The ratio of progress through the current tick. Guaranteed to be at least zero and unlikely to be more than one.
    pub fn tick_ratio_from_last_tick(&self) -> f32 {
        self.time_since_last_tick().as_secs_f32() / self.interval.as_secs_f32()
    }
}
