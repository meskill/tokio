use super::MAX_SAFE_MILLIS_DURATION;
use crate::time::{Clock, Duration, Instant};
use std::sync::LazyLock;

/// Tick the wheel starts at, instead of 0.
///
/// EXPERIMENT (pgdog #1017): the runtime freeze correlates with 2^30 ms
/// (~12.4 days) of uptime, which is where the wheel first cascades an entry
/// from level 4 into level 5. Waiting 12.4 days per attempt is impractical, so
/// `TOKIO_TIME_START_TICK` shifts the whole tick axis: process start maps to
/// that tick rather than 0, and the crossing happens that much sooner.
///
///     TOKIO_TIME_START_TICK=1073731824   # 2^30 - 10_000, crosses after 10s
///
/// Every consumer of the tick axis works on differences (park durations,
/// deadline deltas), so a uniform offset changes only which wheel slots and
/// levels entries land in - exactly the variable under test.
pub(crate) static START_TICK: LazyLock<u64> = LazyLock::new(|| {
    std::env::var("TOKIO_TIME_START_TICK")
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(0)
        .min(MAX_SAFE_MILLIS_DURATION)
});

/// A structure which handles conversion from Instants to `u64` timestamps.
#[derive(Debug)]
pub(crate) struct TimeSource {
    start_time: Instant,
}

impl TimeSource {
    pub(crate) fn new(clock: &Clock) -> Self {
        Self {
            start_time: clock.now(),
        }
    }

    pub(crate) fn deadline_to_tick(&self, t: Instant) -> u64 {
        // Round up to the end of a ms
        self.instant_to_tick(t + Duration::from_nanos(999_999))
    }

    pub(crate) fn instant_to_tick(&self, t: Instant) -> u64 {
        // round up
        let dur: Duration = t.saturating_duration_since(self.start_time);
        let ms = dur
            .as_millis()
            .try_into()
            .unwrap_or(MAX_SAFE_MILLIS_DURATION);
        ms.saturating_add(*START_TICK).min(MAX_SAFE_MILLIS_DURATION)
    }

    pub(crate) fn tick_to_duration(&self, t: u64) -> Duration {
        Duration::from_millis(t)
    }

    pub(crate) fn now(&self, clock: &Clock) -> u64 {
        self.instant_to_tick(clock.now())
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(super) fn start_time(&self) -> Instant {
        self.start_time
    }
}
