use std::time::Instant;

const ALPHA: f64 = 0.25;

pub struct EwmaTracker {
    last_bytes: u64,
    last_time: Instant,
    speed_bps: f64,
}

impl EwmaTracker {
    pub fn new() -> Self {
        Self { last_bytes: 0, last_time: Instant::now(), speed_bps: 0.0 }
    }

    pub fn update(&mut self, total_bytes: u64) -> f64 {
        let now = Instant::now();
        let dt = now.duration_since(self.last_time).as_secs_f64();
        if dt < 0.05 {
            return self.speed_bps;
        }
        let delta = total_bytes.saturating_sub(self.last_bytes) as f64;
        let instant_speed = delta / dt;
        self.speed_bps = ALPHA * instant_speed + (1.0 - ALPHA) * self.speed_bps;
        self.last_bytes = total_bytes;
        self.last_time = now;
        self.speed_bps
    }
}

impl Default for EwmaTracker {
    fn default() -> Self {
        Self::new()
    }
}
