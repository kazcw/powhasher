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
    last_job: Option<JobId>,
    stats: RequestLogger,
}

/*
{
    // getcur blocks until even
    let mut id = getcur();
    let lastlast = last;
    while last != id {
        data = getdata();
        last = id;
        id = getcur();
    }
    updated == (last == lastlast)
}
*/

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
    pub fn get_new_work(&mut self) -> Option<(Target, JobBlob)> {
        let current = self.work.lock().unwrap();
        if let Some(last_job) = self.last_job {
            if last_job == current.job_id {
                return None;
            }
        }
        self.last_job = Some(current.job_id);
        Some((current.target, current.blob.clone()))
    }

    pub fn submit(&mut self, nonce: Nonce, result: &Hash) -> ClientResult<()> {
        let request_id = self.pool
            .lock()
            .unwrap()
            .submit(&self.last_job.unwrap(), nonce, result)?;
        // TODO: errors
        self.stats.share_submitted(request_id).unwrap();
        Ok(())
    }
}
