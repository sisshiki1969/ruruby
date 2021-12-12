use crate::*;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct MethodPerf {
    inline_hit: usize,
    inline_missed: usize,
    total: usize,
    missed: usize,
    timer: std::time::Instant,
    prev_time: Duration,
    prev_method: Option<FnId>,
}

impl MethodPerf {
    pub(crate) fn new() -> Self {
        Self {
            inline_hit: 0,
            inline_missed: 0,
            total: 0,
            missed: 0,
            timer: std::time::Instant::now(),
            prev_time: Duration::from_secs(0),
            prev_method: None,
        }
    }

    pub(crate) fn inc_inline_hit(&mut self) {
        self.inline_hit += 1;
    }

    pub(crate) fn inc_inline_missed(&mut self) {
        self.inline_missed += 1;
    }

    pub(crate) fn inc_total(&mut self) {
        self.total += 1;
    }

    pub(crate) fn inc_missed(&mut self) {
        self.missed += 1;
    }

    pub(crate) fn next(&mut self, method: FnId) -> (Duration, Option<FnId>) {
        let elapsed = self.timer.elapsed();
        let prev = self.prev_time;
        let prev_method = self.prev_method;
        self.prev_time = elapsed;
        self.prev_method = Some(method);
        (elapsed - prev, prev_method)
    }

    pub(crate) fn clear_stats(&mut self) {
        self.inline_hit = 0;
        self.inline_missed = 0;
        self.total = 0;
        self.missed = 0;
    }

    pub(crate) fn print_cache_stats(&self) {
        eprintln!("+-------------------------------------------+");
        eprintln!("| Method cache stats:                       |");
        eprintln!("+-------------------------------------------+");
        eprintln!("  hit inline cache    : {:>10}", self.inline_hit);
        eprintln!("  missed inline cache : {:>10}", self.inline_missed);
        eprintln!("  hit global cache    : {:>10}", self.total - self.missed);
        eprintln!("  missed              : {:>10}", self.missed);
    }
}

#[derive(Debug, Clone)]
pub struct MethodRepoCounter {
    count: usize,
    duration: Duration,
}

impl std::default::Default for MethodRepoCounter {
    fn default() -> Self {
        Self {
            count: 0,
            duration: Duration::from_secs(0),
        }
    }
}

impl MethodRepoCounter {
    pub(crate) fn count(&self) -> usize {
        self.count
    }

    pub(crate) fn count_inc(&mut self) {
        self.count += 1;
    }

    pub(crate) fn duration(&self) -> Duration {
        self.duration
    }

    pub(crate) fn duration_inc(&mut self, dur: Duration) {
        self.duration += dur;
    }
}
