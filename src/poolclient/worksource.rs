// copyright 2017 Kaz Wesley

use job::{Hash, Job, JobBlob, JobId, Nonce, Target};
use poolclient::connection::{ClientResult, PoolClientWriter};
use std::sync::{Arc, Mutex};

/// handle to a source of Jobs and a destination for resulting Shares
#[derive(Clone)]
pub struct WorkSource {
    // XXX: should use a "single-writer seqlock" for work
    work: Arc<Mutex<Job>>,
    pool: Arc<Mutex<PoolClientWriter>>,
    last_job: Option<JobId>,
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
    ) -> WorkSource {
        WorkSource {
            work,
            pool,
            last_job: Default::default(),
        }
    }

    /// return the current work blob if it's newer than the previously-returned
    pub fn get_new_work(&mut self) -> Option<(Target, JobBlob, String)> {
        let current = self.work.lock().unwrap();
        if let Some(last_job) = self.last_job {
            if last_job == current.job_id {
                return None;
            }
        }
        self.last_job = Some(current.job_id);
        Some((current.target, current.blob.clone(), current.algo.clone().unwrap_or("cn/1".to_owned()).to_owned()))
    }

    pub fn submit(&mut self, algo: &str, nonce: Nonce, result: &Hash) -> ClientResult<()> {
        self.pool
            .lock()
            .unwrap()
            .submit(algo, &self.last_job.unwrap(), nonce, result)?;
        Ok(())
    }
}
