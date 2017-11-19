// copyright 2017 Kaz Wesley

use poolclient::connection::RequestId;
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
    expired: AtomicUsize,
    _pad1: [usize; 5],
}

/// a snapshot of poolclient stats
#[derive(Debug)]
pub struct Stats {
    submitted: usize,
    accepted: usize,
    rejected: usize,
    expired: usize,
}

// error not possible until I implement request tracking
#[derive(Debug)]
pub enum RequestStateError {}
pub type RequestStateResult<T> = Result<T, RequestStateError>;

#[derive(Clone)]
pub struct RequestLogger {
    stats: Arc<RequestState>,
//    inflight: spsc::Sender<(RequestId, Instant)>,
}
impl RequestLogger {
    /// RequestId must not already be in-flight; this won't be a
    /// problem as long as RequestIds don't wrap faster than the
    /// request expiration timeout.
    pub fn share_submitted(&self, id: RequestId) -> RequestStateResult<()> {
        //        self.inflight.send((id, Instant::now())).unwrap();
        self.stats.submitted.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
}

pub struct ReplyLogger {
    stats: Arc<RequestState>,
//    inflight: spsc::Receiver<(RequestId, Instant)>,
//    waiting: VecDeque<(RequestId, Instant)>,
}
impl ReplyLogger {
    /*
    fn match_id(&mut self, id: RequestId) -> bool {
        /// TODO: check for expirations
        if let Some(i) = self.waiting.iter().position(|&x| *x.0 == id) {
            let t = self.waiting.remove(i).unwrap().1;
            /// TODO: log response time
            return true;
        }
        let mut found = false;
        self.waiting.extend(self.inflight.try_iter().filter(|x| {
            if !found && (*x.0 == id) {
                found = true;
                let t = self.waiting.remove(i).unwrap().1;
                /// TODO: log response time
                false
            } else {
                true
            }
        }));
        return found;
    }
    */

    /// succeeds unless id is not currently in flight
    pub fn share_accepted(&mut self, id: RequestId) -> RequestStateResult<()> {
        //if !self.match_id { return Err(RequestStateError); }
        self.stats.accepted.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// succeeds unless RequestId is not currently in flight
    pub fn share_rejected(&mut self, id: RequestId) -> RequestStateResult<()> {
        //if !self.match_id { return Err(RequestStateError); }
        self.stats.rejected.fetch_add(1, Ordering::Relaxed);
        Ok(())
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
        let expired = self.0.expired.load(Ordering::Acquire);
        // this load must not be reordered before above loads
        let submitted = self.0.submitted.load(Ordering::Acquire);
        Stats {
            submitted,
            accepted,
            rejected,
            expired,
        }
    }
}

impl Stats {
    /// Shares we have submitted and are expecting a response for.
    pub fn inflight(&self) -> usize {
        self.submitted - self.accepted - self.rejected - self.expired
    }

    pub fn submitted(&self) -> usize {
        self.submitted
    }

    pub fn accepted(&self) -> usize {
        self.accepted
    }

    /// Shares we received an error response for, typically stale
    /// shares. High values suggest latency to pool is too high, or
    /// invalid shares are being generated. See pool error messages
    /// for details.
    pub fn rejected(&self) -> usize {
        self.rejected
    }

    /// Shares we never received a response for (NOT stale shares).
    /// This shouldn't generally happen, and would suggest message
    /// corruption or a pool error.
    pub fn expired(&self) -> usize {
        self.expired
    }
}

pub fn request_state_tracker() -> (RequestLogger, ReplyLogger, StatReader) {
    let stats = Arc::new(Default::default());
    //let (sender, receiver) = spsc::channel();
    (
        RequestLogger {
            stats: Arc::clone(&stats),
//            inflight: sender
        },
        ReplyLogger {
            stats: Arc::clone(&stats),
//            inflight: receiver
        },
        StatReader(stats),
    )
}
