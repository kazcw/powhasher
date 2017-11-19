// copyright 2017 Kaz Wesley

use job::{Hash, Job, JobBlob, JobId, Nonce, Target};
use poolclient::connection::{ClientResult, PoolClientWriter};
use poolclient::stats::RequestLogger;
use std::sync::{Arc, Mutex};

/// handle to a source of Jobs and a destination for resulting Shares
#[derive(Clone)]
pub struct WorkSource {
    // XXX: should use a "single-writer seqlock" for work
    work: Arc<Mutex<Job>>,
    pool: Arc<Mutex<PoolClientWriter>>,
    last_job: Option<(JobId, Target)>,
    stats: RequestLogger,
}

impl WorkSource {
    pub fn new(
        work: Arc<Mutex<Job>>,
        pool: Arc<Mutex<PoolClientWriter>>,
        stats: RequestLogger,
    ) -> WorkSource {
        WorkSource {
            work,
            pool,
            last_job: Default::default(),
            stats,
        }
    }

    /// return the current work blob if it's newer than the previously-returned
    pub fn get_new_work(&mut self) -> Option<JobBlob> {
        let current = self.work.lock().unwrap();
        if let Some(last_job) = self.last_job {
            if last_job.0 == current.job_id {
                return None;
            }
        }
        self.last_job = Some((current.job_id, current.target));
        Some(current.blob.clone())
    }

    pub fn result(&mut self, nonce: Nonce, result: &Hash) -> ClientResult<()> {
        // TODO: handle state error gracefully
        if self.last_job.unwrap().1.is_hit(result) {
            let request_id = self.pool.lock().unwrap().submit(
                &self.last_job.unwrap().0,
                nonce,
                result,
            )?;
            // TODO: errors
            self.stats.share_submitted(request_id).unwrap();
        }
        Ok(())
    }
}
