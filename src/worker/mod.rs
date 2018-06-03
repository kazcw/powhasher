// copyright 2017 Kaz Wesley

pub mod stats;

use core_affinity::{self, CoreId};
use cryptonight::{self, HasherConfig};
use job::{CpuId, Hash, Nonce};
use poolclient::WorkSource;
use self::stats::StatUpdater;

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
        let (mut target, blob, mut algo) = self.worksource.get_new_work().unwrap();
        let mut blob = blob.0;
        let mut start = ((blob[42] as u32) << 24) + worker_id;
        let mut hashes = cryptonight::hasher(&algo, cfg.hasher, blob, (start..).step_by(step as usize));
        loop {
            let mut nonces = (start..).step_by(step as usize).map(Nonce);
            loop {
                let ws = &mut self.worksource;
                (hashes
                    .by_ref()
                    .take(SINGLEHASH_BATCH_SIZE)
                    .map(Hash::new)
                    .zip(nonces.by_ref())
                    .filter(|(h, _n)| target.is_hit(h))
                    .map(|(h, n)| ws.submit(&algo, n, &h))
                    .collect(): Result<Vec<_>, _>)
                    .unwrap();
                self.stat_updater.log_hashes(SINGLEHASH_BATCH_SIZE);
                if let Some((newt, newb, newa)) = ws.get_new_work() {
                    target = newt;
                    blob = newb.0;
                    algo = newa;
                    break;
                }
            }
            start = ((blob[42] as u32) << 24) + worker_id;
            let noncer = (start..).step_by(step as usize);
            hashes.set_blob(blob, noncer);
        }
    }
}
