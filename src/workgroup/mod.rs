mod worker;
mod stats;

use self::stats::StatReader;
pub use self::worker::Worker;
use core_affinity::{self, CoreId};
use hasher::HasherBuilder;
use poolclient::WorkSource;
use std::thread;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config(Vec<worker::Config>);

pub struct Workgroup {
    worksource: WorkSource,
    hasher_builder: HasherBuilder,
    core_ids: Vec<CoreId>,
}

impl Workgroup {
    pub fn new(worksource: WorkSource, hasher_builder: HasherBuilder) -> Self {
        Workgroup {
            worksource,
            hasher_builder,
            core_ids: core_affinity::get_core_ids().unwrap(),
        }
    }

    fn run_thread(&self, worker_id: usize, cfg: worker::Config) -> StatReader {
        let (stat_updater, stat_reader) = stats::channel();
        let worker = Worker::new(self.worksource.clone(), stat_updater);
        let hasher_builder = self.hasher_builder.clone();
        let core_ids = self.core_ids.clone();
        debug!("starting worker{} with config: {:?}", worker_id, &cfg);
        thread::Builder::new()
            .name(format!("worker{}", worker_id))
            .spawn(move || worker.run(cfg, hasher_builder, core_ids))
            .unwrap();
        stat_reader
    }

    pub fn run_threads(self, cfg: Config) -> Vec<StatReader> {
        cfg.0
            .into_iter()
            .enumerate()
            .map(|(i, w)| self.run_thread(i, w))
            .collect()
    }
}
