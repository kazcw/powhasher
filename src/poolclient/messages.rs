// copyright 2017 Kaz Wesley

//! Serialization for the JSON-RPC-based `CryptoNote` pool protocol

use crate::poolclient::hexbytes;

use arrayvec::ArrayString;
use serde::Deserializer;
use serde_derive::{Deserialize, Serialize};

use std::error::Error;
use std::fmt::{self, Display, Formatter};

////////// COMMON

/// `WorkerId` can be any JSON string of up to 64 bytes. It is opaque to
/// the worker.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WorkerId(ArrayString<[u8; 64]>);

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct JobId(ArrayString<[u8; 64]>);

////////// server -> worker

// Input is either 32-bit or 64-bit little-endian hex string, not necessarily padded.
// Inputs of 8 hex chars or less are in a compact format.
pub fn deserialize_target<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let (mut val, hexlen) = hexbytes::hex64le_to_int(deserializer)?;
    // unpack compact format
    // XXX: this is what other miners do. It doesn't seem right...
    if hexlen <= 8 {
        val |= val << 0x20;
    }
    Ok(val)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Job {
    #[serde(deserialize_with = "hexbytes::hex_to_varbyte")]
    pub blob: Vec<u8>,
    pub job_id: JobId,
    #[serde(deserialize_with = "deserialize_target")]
    pub target: u64,
    #[serde(default)]
    pub algo: Option<String>,
    #[serde(default)]
    variant: u32, // xmrig sends this for compat with obsolete xmrig
}

impl PartialEq<Job> for Job {
    fn eq(&self, other: &Job) -> bool {
        self.job_id == other.job_id
    }
}

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
    #[serde(serialize_with = "hexbytes::u32_to_hex_padded")]
    pub nonce: u32,
    #[serde(serialize_with = "hexbytes::byte32_to_hex")]
    pub result: [u8; 32],
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
