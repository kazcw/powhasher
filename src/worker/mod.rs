// copyright 2017 Kaz Wesley

pub mod stats;

use core_affinity::{self, CoreId};
use cryptonight::{self, Hasher, HasherConfig};
use job::{CpuId, Hash, Nonce};
use poolclient::WorkSource;
use stats::StatUpdater;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    cpu: CpuId,
    hasher: HasherConfig,
}

pub struct Worker {
    worksource: WorkSource,
    stat_updater: StatUpdater,
}

/// Number of hashes to do in a batch, i.e. between checks for new work.
const SINGLEHASH_BATCH_SIZE: usize = 16;

impl Worker {
    pub fn new(worksource: WorkSource, stat_updater: StatUpdater) -> Self {
        Worker {
            worksource,
            stat_updater,
        }
    }

    pub fn run(mut self, cfg: Config, core_ids: Vec<CoreId>, worker_id: u32, step: u32) -> ! {
        // TODO: CoreId error handling
        core_affinity::set_for_current(core_ids[cfg.cpu.0 as usize]);
        self.stat_updater.reset();
        let (mut target, blob) = self.worksource.get_new_work().unwrap();
        let mut blob = blob.0;
        let mut start = ((blob[42] as u32) << 24) + worker_id;
        let mut hashes = cryptonight::hasher(cfg.hasher, blob, (start..).step_by(step as usize));
        loop {
            let mut nonces = (start..).step_by(step as usize).map(Nonce);
            loop {
                let ws = &mut self.worksource;
                let mut ct = 0;
                (hashes
                    .by_ref()
                    .take(SINGLEHASH_BATCH_SIZE)
                    .map(Hash::new)
                    .inspect(|_| ct += 1)
                    .zip(nonces.by_ref())
                    .filter(|(h, n)| target.is_hit(h))
                    .map(|(h, n)| ws.submit(n, &h))
                    .collect(): Result<Vec<_>, _>)
                    .unwrap();
                self.stat_updater.log_hashes(ct);
                if let Some((newt, newb)) = ws.get_new_work() {
                    target = newt;
                    blob = newb.0;
                    break;
                }
            }
            start = ((blob[42] as u32) << 24) + worker_id;
            let noncer = (start..).step_by(step as usize);
            hashes.set_blob(blob, noncer);
        }
    }
}
