// copyright 2017 Kaz Wesley

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Current poolclient stats.
///
/// Consistency: the only guarantee maintained is that if a reply is
/// counted in `accepted`/`rejected`, the request is counted in
/// `submitted`. More formally, it must not be possible to observe a
/// state such that `accepted` + `rejected` > `submitted`.
///
/// This guarantee cannot be maintained internally: this impl can only
/// guarantee that it will be upheld so long as all of the following
/// hold:
/// - `inc_submitted` is called *before* sending off a share
/// - `inc_accepted`/`inc_rejected` are called no more than once per share *submitted*
#[derive(Default)]
pub struct RequestState {
    /// must be incremented before share submission
    submitted: AtomicUsize,
    _pad0: [usize; 7],

    accepted: AtomicUsize,
    rejected: AtomicUsize,
    _pad1: [usize; 6],
}

/// a snapshot of poolclient stats
#[derive(Debug)]
pub struct Stats {
    submitted: usize,
    accepted: usize,
    rejected: usize,
}

#[derive(Clone)]
pub struct RequestLogger {
    stats: Arc<RequestState>,
}
impl RequestLogger {
    pub fn share_submitted(&self) {
        self.stats.submitted.fetch_add(1, Ordering::AcqRel);
    }
}

pub struct ReplyLogger {
    stats: Arc<RequestState>,
}
impl ReplyLogger {
    pub fn share_accepted(&mut self) {
        self.stats.accepted.fetch_add(1, Ordering::Relaxed);
    }

    pub fn share_rejected(&mut self) {
        self.stats.rejected.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Clone)]
pub struct StatReader(Arc<RequestState>);
impl StatReader {
    pub fn get(&self) -> Stats {
        // similar synchronization needs to seqlock here
        // rel/acq prevents reordering more or less for free on x86
        let accepted = self.0.accepted.load(Ordering::Acquire);
        let rejected = self.0.rejected.load(Ordering::Acquire);
        // this load must not be reordered before above loads
        let submitted = self.0.submitted.load(Ordering::Acquire);
        Stats {
            submitted,
            accepted,
            rejected,
        }
    }
}

pub fn request_state_tracker() -> (RequestLogger, ReplyLogger, StatReader) {
    let stats = Arc::new(Default::default());
    //let (sender, receiver) = spsc::channel();
    (
        RequestLogger {
            stats: Arc::clone(&stats),
        },
        ReplyLogger {
            stats: Arc::clone(&stats),
        },
        StatReader(stats),
    )
}
