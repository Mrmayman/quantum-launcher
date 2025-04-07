use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};

use tokio::sync::Mutex;

pub static RATE_LIMITER: LazyLock<RateLimiter> = LazyLock::new(RateLimiter::default);
pub static MOD_DOWNLOAD_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub struct RateLimiter {
    last_executed: Mutex<Instant>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            last_executed: Mutex::new(Instant::now() - Self::DELAY),
        }
    }
}

impl RateLimiter {
    // 200ms delay duration
    const DELAY: Duration = Duration::from_millis(200);

    pub async fn lock(&self) {
        let mut last_exec_time = self.last_executed.lock().await;
        let now = Instant::now();

        let elapsed = now.duration_since(*last_exec_time);

        if elapsed < Self::DELAY {
            let wait_duration = Self::DELAY - elapsed;
            tokio::time::sleep(wait_duration).await;
        }

        // Update the last execution time to now
        *last_exec_time = Instant::now();
    }
}
