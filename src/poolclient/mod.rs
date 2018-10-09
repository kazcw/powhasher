// copyright 2017 Kaz Wesley

//! application layer of pool client

mod connection;
mod messages;
mod stats;
mod worksource;

use self::connection::{PoolClientReader, PoolClientWriter, RequestId};
pub use self::connection::ClientResult;
use self::messages::{ClientCommand, PoolEvent, PoolReply};
pub use self::stats::{ReplyLogger, RequestLogger, RequestState, StatReader, Stats};
pub use self::worksource::WorkSource;
use job::Job;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub address: String,
    pub login: String,
    pub pass: String,
    pub keepalive_s: Option<u64>,
}

struct PoolClient {
    writer: Arc<Mutex<PoolClientWriter>>,
    reader: PoolClientReader,
    replies: ReplyLogger,
    work: Arc<Mutex<Job>>,
}

impl PoolClient {
    fn handle(&mut self, event: PoolEvent<RequestId>) {
        match event {
            PoolEvent::ClientCommand(c) => match c {
                ClientCommand::Job(j) => {
                    debug!("new job: {:?}", j);
                    *self.work.lock().unwrap() = j;
                }
            },
            PoolEvent::PoolReply { id: _id, error: Some(error), .. } => {
                warn!(
                    "received error: {:?}, assuming that indicates a stale share",
                    error
                );
                self.replies.share_rejected();
            }
            PoolEvent::PoolReply {
                id: _id,
                error: None,
                result: Some(result),
            } => {
                debug!("pool reply");
                match result {
                    PoolReply::Status { status } => {
                        if status == "OK" {
                            debug!("received status OK");
                        } else {
                            info!("received status {:?}, assuming that means OK", status);
                        }
                        self.replies.share_accepted();
                    }
                    PoolReply::Job { .. } => {
                        warn!("unexpected job reply...");
                    }
                };
                // TODO
            }
            PoolEvent::PoolReply {
                id: _id,
                error: None,
                result: None,
            } => warn!("pool reply with no content"),
        }
    }

    fn run(mut self) -> ClientResult<()> {
        loop {
            if let Some(event) = self.reader.read()? {
                self.handle(event);
            } else {
                debug!("read timeout; sending keepalive");
                self.writer.lock().unwrap().keepalive().unwrap();
            }
        }
    }
}

pub fn run_thread(cfg: &Config, agent: &str) -> ClientResult<(WorkSource, StatReader)> {
    let (writer, work, reader) = connection::connect(
        &cfg.address,
        &cfg.login,
        &cfg.pass,
        agent,
        cfg.keepalive_s.map(Duration::from_secs),
    )?;
    debug!("client connected, initial job: {:?}", &work);
    let work = Arc::new(Mutex::new(work));
    let writer = Arc::new(Mutex::new(writer));
    let (requests, replies, stats) = stats::request_state_tracker();
    let client = PoolClient {
        writer: Arc::clone(&writer),
        reader,
        replies,
        work: Arc::clone(&work),
    };
    thread::Builder::new()
        .name("poolclient".into())
        .spawn(move || client.run())
        .unwrap();
    Ok((WorkSource::new(work, writer, requests), stats))
}
