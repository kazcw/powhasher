// copyright 2017 Kaz Wesley

//! Serialization for the JSON-RPC-based `CryptoNote` pool protocol

use arrayvec::ArrayString;
use job::{Hash, Job, JobId, Nonce};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

////////// COMMON

/// `WorkerId` can be any JSON string of up to 64 bytes. It is opaque to
/// the worker.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WorkerId(ArrayString<[u8; 64]>);

////////// server -> worker

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "lowercase")]
pub enum ClientCommand {
    Job(Job),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorReply {
    code: i64,
    message: String,
}

impl Display for ErrorReply {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", &self)
    }
}

impl Error for ErrorReply {
    fn description(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonMessage<T> {
    #[serde(default)]
    pub jsonrpc: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(flatten)]
    pub body: T,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PoolReply {
    /// reply to getjob (not implemented) and login
    Job {
        #[serde(rename = "id")]
        worker_id: WorkerId,
        job: Job,
        #[serde(default)]
        status: Option<String>,
        #[serde(default)]
        extensions: Vec<String>,
    },
    /// reply to submit
    Status { status: String },
}

/// Message received from pool (reply or job notification).
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PoolEvent<ReqId> {
    ClientCommand(ClientCommand),
    PoolReply {
        id: ReqId,
        error: Option<ErrorReply>,
        result: Option<PoolReply>,
    },
}

////////// worker -> server

#[derive(Debug, Serialize, Deserialize)]
pub struct Share {
    #[serde(rename = "id")]
    pub worker_id: WorkerId,
    pub job_id: JobId,
    pub nonce: Nonce,
    pub result: Hash,
    pub algo: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Credentials {
    pub login: String,
    pub pass: String,
    pub agent: String,
    pub algo: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "lowercase")]
pub enum PoolCommand {
    Submit(Share),
    Login(Credentials),
    KeepAlived{ id: WorkerId },
}

/// Message sent from client to pool.
///
/// `ReqId` can be any JSON value. If you are sending the requests, you
/// can serialize with a specific type like u32, and should be able to
/// expect the same type to come back in replies. If you are receiving
/// the requests, you should use a generic type like
/// `serde_json::Value`.
#[derive(Debug, Serialize, Deserialize)]
pub struct PoolRequest<ReqId> {
    pub id: ReqId,
    #[serde(flatten)]
    pub command: PoolCommand,
}
