//! Belirteç kovası hız sınırlayıcı. Genel ve indirme başına sınır birlikte uygulanır.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

pub struct RateLimiter {
    /// Bayt/sn; 0 sınırsız.
    rate: AtomicU64,
    state: Mutex<(f64, Instant)>,
}

impl RateLimiter {
    pub fn new(rate: u64) -> Self {
        Self { rate: AtomicU64::new(rate), state: Mutex::new((0.0, Instant::now())) }
    }

    pub fn set_rate(&self, rate: u64) {
        self.rate.store(rate, Ordering::Relaxed);
    }

    pub fn rate(&self) -> u64 {
        self.rate.load(Ordering::Relaxed)
    }

    /// `n` bayt için gereken bekleme süresini ayırır ve döndürür.
    fn reserve(&self, n: u64) -> Duration {
        let rate = self.rate();
        if rate == 0 {
            return Duration::ZERO;
        }
        let mut st = self.state.lock();
        let now = Instant::now();
        let elapsed = now.duration_since(st.1).as_secs_f64();
        // Kova en çok 0,25 sn'lik veri biriktirir; ani patlamalar sınırlı kalır.
        let cap = rate as f64 * 0.25;
        st.0 = (st.0 + elapsed * rate as f64).min(cap);
        st.1 = now;
        st.0 -= n as f64;
        if st.0 >= 0.0 {
            Duration::ZERO
        } else {
            Duration::from_secs_f64((-st.0) / rate as f64)
        }
    }

    pub async fn acquire(&self, n: u64) {
        let wait = self.reserve(n);
        if !wait.is_zero() {
            tokio::time::sleep(wait.min(Duration::from_secs(5))).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn limits_rate() {
        let l = RateLimiter::new(100_000);
        let t = Instant::now();
        for _ in 0..20 {
            l.acquire(10_000).await;
        }
        // 200 KB / 100 KB/s ≈ 2 sn (ilk 25 KB kovadan)
        let e = t.elapsed().as_secs_f64();
        assert!(e > 1.5 && e < 2.6, "geçen süre {e}");
    }
}
