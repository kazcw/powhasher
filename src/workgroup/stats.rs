// copyright 2017 Kaz Wesley

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Minimum increase to submit to statistic counter.
const LOG_THRESHOLD: usize = 16;

#[derive(Default, Copy, Clone, Debug)]
pub struct WorkerStats {
    pub hashes: usize,
    pub runtime: Duration,
}

pub struct StatUpdater {
    stats: Arc<Mutex<WorkerStats>>,
    new_hashes: usize,
    logged_hashes: usize,
    start_time: Instant,
}

// TODO: replace new/reset with builder that starts timer once, at the right time
impl StatUpdater {
    fn new(stats: Arc<Mutex<WorkerStats>>) -> Self {
        StatUpdater {
            stats,
            new_hashes: 0,
            logged_hashes: 0,
            start_time: Instant::now(),
        }
    }

    pub fn reset(&mut self) {
        self.start_time = Instant::now();
    }

    pub fn log_hashes(&mut self, n: usize) {
        self.new_hashes += n;
        if self.new_hashes >= LOG_THRESHOLD {
            self.logged_hashes += self.new_hashes;
            self.new_hashes = 0;
            let dur = self.start_time.elapsed();
            let mut stats = self.stats.lock().unwrap();
            stats.hashes = self.logged_hashes;
            stats.runtime = dur;
        }
    }
}

pub struct StatReader(Arc<Mutex<WorkerStats>>);

impl StatReader {
    pub fn get(&self) -> WorkerStats {
        *self.0.lock().unwrap()
    }
}

pub fn channel() -> (StatUpdater, StatReader) {
    let stats = Arc::new(Mutex::new(WorkerStats::default()));
    let writer = StatUpdater::new(Arc::clone(&stats));
    let reader = StatReader(stats);
    (writer, reader)
}
