// copyright 2017 Kaz Wesley

pub mod stats;

use core_affinity::{self, CoreId};
use cryptonight::CryptoNight;
use job::{CpuId, Hash, Nonce};
use poolclient::WorkSource;
use stats::StatUpdater;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Hasher {
    #[serde(rename = "cn-cpu-aesni")]
    CnCpuAesni { multi: usize },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    cpu: CpuId,
    hasher: Hasher,
}

pub struct Worker {
    worksource: WorkSource,
    stat_updater: StatUpdater,
}

/// Number of hashes to do in a batch, i.e. between checks for new work.
const SINGLEHASH_BATCH_SIZE: usize = 16;

fn set_nonce(v: &mut [u8], nonce: u32) {
    v[39] = (nonce >> 0x18) as u8;
    v[40] = (nonce >> 0x10) as u8;
    v[41] = (nonce >> 0x08) as u8;
    v[42] = (nonce >> 0x00) as u8;
}

impl Worker {
    pub fn new(worksource: WorkSource, stat_updater: StatUpdater) -> Self {
        Worker {
            worksource,
            stat_updater,
        }
    }

    pub fn run(mut self, cfg: Config, core_ids: Vec<CoreId>) -> ! {
        // TODO: CoreId error handling
        core_affinity::set_for_current(core_ids[cfg.cpu.0 as usize]);
        self.stat_updater.reset();
        let base_nonce = (cfg.cpu.into(): Nonce).0;
        let mut blob = self.worksource.get_new_work().unwrap().0;
        let mut state = CryptoNight::new();
        loop {
            let mut nonce = base_nonce;
            set_nonce(&mut blob, nonce);
            state.init(&blob);
            loop {
                for _ in 0..SINGLEHASH_BATCH_SIZE {
                    let prev_nonce = nonce;
                    nonce = nonce.wrapping_add(1);
                    set_nonce(&mut blob, nonce);
                    let prev_result = state.advance(&blob);
                    self.worksource
                        .result(Nonce(prev_nonce), &Hash::new(prev_result))
                        .unwrap()
                }

                self.stat_updater.log_hashes(SINGLEHASH_BATCH_SIZE);

                if let Some(new_blob) = self.worksource.get_new_work() {
                    blob = new_blob.0;
                    break;
                }
            }
        }
    }
}
