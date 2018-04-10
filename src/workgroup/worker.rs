// copyright 2017 Kaz Wesley

use core_affinity::{self, CoreId};
use hasher::{self, HasherBuilder};
use job::{CpuId, Nonce};
use poolclient::WorkSource;
use workgroup::stats::StatUpdater;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    cpu: CpuId,
    hasher: hasher::Config,
}

pub struct Worker {
    worksource: WorkSource,
    stat_updater: StatUpdater,
}

impl Worker {
    pub fn new(worksource: WorkSource, stat_updater: StatUpdater) -> Self {
        Worker {
            worksource,
            stat_updater,
        }
    }

    pub fn run(mut self, cfg: Config, hasher_builder: HasherBuilder, core_ids: Vec<CoreId>) -> ! {
        // TODO: CoreId error handling
        core_affinity::set_for_current(core_ids[cfg.cpu.0 as usize]);
        let base_nonce: Nonce = cfg.cpu.into();
        let mut hasher = hasher_builder.into_hasher(&cfg.hasher, base_nonce.0);
        self.stat_updater.reset();
        let mut job = self.worksource.get_new_work().unwrap();
        loop {
            let mut hashes = hasher.hashes(job.0);
            job = loop {
                let n = hashes.run_batch(&mut |nonce, hash| {
                    self.worksource.result(Nonce(nonce), hash).unwrap()
                });
                self.stat_updater.log_hashes(n);

                if let Some(new_job) = self.worksource.get_new_work() {
                    break new_job;
                }
            }
        }
    }
}
