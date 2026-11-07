//! Async token-bucket limiter shared by every connection of every file.

use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    rate: f64, // bytes per second
    state: Mutex<(f64, Instant)>,
}

impl RateLimiter {
    pub fn new(bytes_per_sec: f64) -> Self {
        Self { rate: bytes_per_sec, state: Mutex::new((bytes_per_sec, Instant::now())) }
    }

    /// Wait until `n` bytes of budget are available. Tokens may go negative ("debt") when a
    /// single read is larger than the one-second bucket; the sleep repays it at `rate`.
    pub async fn consume(&self, n: usize) {
        if n == 0 {
            return;
        }
        let deficit = {
            let mut s = self.state.lock().unwrap();
            let now = Instant::now();
            let elapsed = now.duration_since(s.1).as_secs_f64();
            s.1 = now;
            s.0 = (s.0 + elapsed * self.rate).min(self.rate) - n as f64;
            -s.0
        };
        if deficit > 0.0 {
            tokio::time::sleep(Duration::from_secs_f64(deficit / self.rate)).await;
        }
    }
}
