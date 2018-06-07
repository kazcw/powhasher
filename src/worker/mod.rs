// copyright 2017 Kaz Wesley

pub mod stats;

use core_affinity::{self, CoreId};
use cryptonight::{self, HasherConfig};
use poolclient::WorkSource;
use self::stats::StatUpdater;
use std::convert::TryFrom;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    cpu: u32,
    hasher: HasherConfig,
}

pub struct Worker {
    worksource: WorkSource,
    stat_updater: StatUpdater,
}

/// Number of hashes to do in a batch, i.e. between checks for new work.
const SINGLEHASH_BATCH_SIZE: usize = 1;

fn is_hit(hash: &[u8; 32], target: u64) -> bool {
    let hashle64 = hash[24..].iter().enumerate().fold(0u64, |x, (i, v)| {
        x | u64::from(*v) << (i * 8)
    });
    hashle64 <= target
}

impl Worker {
    pub fn new(worksource: WorkSource, stat_updater: StatUpdater) -> Self {
        Worker {
            worksource,
            stat_updater,
        }
    }

    pub fn run(mut self, cfg: Config, core_ids: Vec<CoreId>, worker_id: u32, step: u32) -> ! {
        // TODO: CoreId error handling
        core_affinity::set_for_current(core_ids[cfg.cpu as usize]);
        self.stat_updater.reset();
        let (mut target, blob, algo) = self.worksource.get_new_work().unwrap();
        let start = ((blob[42] as u32) << 24) + worker_id;
        let mut hashes = cryptonight::hasher(&algo, &cfg.hasher, blob, (start..).step_by(step as usize));
        loop {
            let mut nonces = (start..).step_by(step as usize);
            loop {
                let ws = &mut self.worksource;
                (hashes
                    .by_ref()
                    .take(SINGLEHASH_BATCH_SIZE)
                    .map(|h| *<&[u8; 32]>::try_from(h.as_slice()).unwrap())
                    .zip(nonces.by_ref())
                    .filter(|(h, _n)| is_hit(h, target))
                    .map(|(h, n)| ws.submit(&algo, n, &h))
                    .collect(): Result<Vec<_>, _>)
                    .unwrap();
                self.stat_updater.log_hashes(SINGLEHASH_BATCH_SIZE);
                if let Some((newt, newb, newa)) = ws.get_new_work() {
                    hashes = cryptonight::hasher(&newa, &cfg.hasher, newb, (start..).step_by(step as usize));
                    target = newt;
                    break;
                }
            }
        }
    }
}
