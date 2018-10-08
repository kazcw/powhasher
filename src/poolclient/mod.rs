// copyright 2017 Kaz Wesley

//! application layer of pool client

mod connection;
mod hexbytes;
mod messages;

use self::connection::PoolClientReader;
use self::messages::{PoolReply, PoolEvent, ClientCommand};

pub use self::connection::{ClientResult, RequestId, PoolClientWriter};
pub use self::messages::{ErrorReply, Job, JobAssignment, JobId};

use std::sync::{Arc, Mutex};
use std::time::Duration;

use log::*;
use serde_derive::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub address: String,
    pub login: String,
    pub pass: String,
    pub keepalive_s: Option<u64>,
}

pub trait MessageHandler {
    fn job_command(&mut self, job: Job);
    fn error_reply(&mut self, id: RequestId, error: ErrorReply);
    fn status_reply(&mut self, id: RequestId, status: String);
    fn job_reply(&mut self, id: RequestId, job: Box<JobAssignment>);
}

pub struct PoolClient<H> {
    writer: Arc<Mutex<PoolClientWriter>>,
    reader: PoolClientReader,
    handler: H,
}

impl<H: MessageHandler> PoolClient<H> {
    pub fn connect<F, X>(cfg: &Config, agent: &str, make_handler: F) -> ClientResult<(Self, X)> where F: FnOnce(Job) -> (H, X) {
        let (writer, work, reader) = connection::connect(
            &cfg.address,
            &cfg.login,
            &cfg.pass,
            agent,
            cfg.keepalive_s.map(Duration::from_secs),
        )?;
        debug!("client connected, initial job: {:?}", &work);
        let writer = Arc::new(Mutex::new(writer));
        let (handler, x) = make_handler(work);
        Ok((PoolClient { writer, reader, handler }, x))
    }

    pub fn write_handle(&self) -> Arc<Mutex<PoolClientWriter>> {
        Arc::clone(&self.writer)
    }

    fn handle(&mut self, event: PoolEvent<RequestId>) {
        match event {
            PoolEvent::ClientCommand(ClientCommand::Job(j)) => self.handler.job_command(j),
            PoolEvent::PoolReply { id, error: Some(error), .. } => self.handler.error_reply(id, error),
            PoolEvent::PoolReply { id, error: None, result: Some(PoolReply::Status { status }) } => self.handler.status_reply(id, status),
            PoolEvent::PoolReply { id, error: None, result: Some(PoolReply::Job(job)) } => self.handler.job_reply(id, job),
            PoolEvent::PoolReply { error: None, result: None, .. } => warn!("pool reply with no content")
        }
    }

    /// Handle messages until the connection is closed.
    pub fn run(mut self) -> ClientResult<()> {
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
