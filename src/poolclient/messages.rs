// copyright 2017 Kaz Wesley

//! Serialization for the JSON-RPC-based `CryptoNote` pool protocol

use arrayvec::ArrayString;
use job::{Hash, Job, JobId, Nonce};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// `WorkerId` can be any JSON string of up to 64 bytes. It is opaque to
/// the worker.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerId(ArrayString<[u8; 64]>);

#[derive(Debug, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "lowercase")]
pub enum ClientCommand {
    Job(Job),
}

#[derive(Debug, Deserialize)]
pub struct ErrorReply {
    code: i64,
    message: String,
}

impl Display for ErrorReply {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", &self)
    }
}

impl ErrorReply {
    fn code(&self) -> i64 {
        self.code
    }
}

impl Error for ErrorReply {
    fn description(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PoolReply {
    /// reply to getjob (not implemented) and login
    Job {
        #[serde(rename = "id")]
        worker_id: WorkerId,
        status: String,
        job: Job,
    },
    /// reply to submit
    Status { status: String },
}

/// Message received from pool (reply or job notification).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PoolEvent<ReqId> {
    ClientCommand(ClientCommand),
    PoolReply {
        id: ReqId,
        error: Option<ErrorReply>,
        result: Option<PoolReply>,
    },
}

#[derive(Debug, Serialize)]
pub struct Share<'a> {
    #[serde(rename = "id")]
    pub worker_id: &'a WorkerId,
    pub job_id: &'a JobId,
    pub nonce: Nonce,
    pub result: &'a Hash,
}

#[derive(Debug, Serialize)]
pub struct Credentials<'a> {
    pub login: &'a str,
    pub pass: &'a str,
    pub agent: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(tag = "method", content = "params", rename_all = "lowercase")]
pub enum PoolCommand<'a> {
    Submit(Share<'a>),
    Login(Credentials<'a>),
}

/// Message sent from client to pool.
///
/// `ReqId` can be any JSON value. If you are sending the requests, you
/// can serialize with a specific type like u32, and should be able to
/// expect the same type to come back in replies. If you are receiving
/// the requests, you should use a generic type like
/// `serde_json::Value`.
#[derive(Debug, Serialize)]
pub struct PoolRequest<'a, ReqId> {
    pub id: ReqId,
    #[serde(flatten)]
    pub command: &'a PoolCommand<'a>,
}
