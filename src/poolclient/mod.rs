// copyright 2017 Kaz Wesley

//! application layer of pool client

mod connection;
mod hexbytes;
mod messages;
mod worksource;

use self::connection::{PoolClientReader, PoolClientWriter, RequestId};
pub use self::connection::ClientResult;
use self::messages::{ClientCommand, PoolEvent, PoolReply, Job};
pub use self::worksource::WorkSource;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use log::{debug, info, log, warn};
use serde_derive::{Serialize, Deserialize};

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

pub fn run_thread(cfg: &Config, agent: &str) -> ClientResult<WorkSource> {
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
    let client = PoolClient {
        writer: Arc::clone(&writer),
        reader,
        work: Arc::clone(&work),
    };
    thread::Builder::new()
        .name("poolclient".into())
        .spawn(move || client.run())
        .unwrap();
    Ok(WorkSource::new(work, writer))
}
