// copyright 2017 Kaz Wesley

// core_affinity (maintained)
// hwloc (maintained)
// serde+serde_json

// libcpuid (rs wrapper broken, needs work/replacement)
// memmap (no hugepage support)

#![feature(alloc_system)]
#![feature(chunks_exact)]

// no allocs on hot paths anyway
extern crate alloc_system;

mod worksource;

use std::fs::File;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Instant, Duration};

use cn_stratum::client::{PoolClient, MessageHandler, RequestId, Job, JobAssignment, ErrorReply};
use crate::worksource::WorkSource;

use log::*;
use serde_derive::{Deserialize, Serialize};
use core_affinity::CoreId;
use byteorder::{ByteOrder, LE};

const AGENT: &str = "pow#er/0.2.0";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientConfig {
    pub address: String,
    pub login: String,
    pub pass: String,
    pub keepalive_s: Option<u64>,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
    pub pool: ClientConfig,
    pub workers: Vec<WorkerConfig>,
}

fn main() {
    env_logger::init();

    let panicker = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        eprintln!("panicked");
        panicker(info);
        std::process::exit(1);
    }));

    let args = clap::App::new("Pow#er")
        .author("Kaz Wesley <kaz@lambdaverse.org>")
        .arg(
            clap::Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .get_matches();

    let cfg: Config = File::open(args.value_of("config").unwrap_or("./config.json"))
        .map(serde_json::from_reader)
        .unwrap()
        .unwrap();
    debug!("config: {:?}", &cfg);

    let worksource = {
        let cfg = &cfg.pool;
        let keepalive = cfg.keepalive_s.map(Duration::from_secs);
        let client = PoolClient::connect(&cfg.address, &cfg.login, &cfg.pass, keepalive, AGENT, Client::new).unwrap();
        let work = client.handler().job_handle();
        let writer = client.write_handle();
        thread::Builder::new()
            .name("poolclient".into())
            .spawn(move || client.run())
            .unwrap();
        WorkSource::new(work, writer)
    };

    let core_ids = core_affinity::get_core_ids().unwrap();
    let worker_count = cfg.workers.len();
    let workerstats: Vec<_> = cfg.workers
        .into_iter()
        .enumerate()
        .map(|(i, w)| {
            let hash_count = Arc::new(AtomicUsize::new(0));
            let hash_counter = Arc::clone(&hash_count);
            let ws = worksource.clone();
            let core = core_ids[w.cpu as usize];
            debug!("starting worker{} with config: {:?}", i, &w);
            thread::Builder::new()
                .name(format!("worker{}", i))
                .spawn(move || run_worker(w, ws, hash_count, core, i as u32, worker_count as u32))
                .unwrap();
            hash_counter
        })
        .collect();

    let mut prevstats: Vec<_> = workerstats.iter().map(|w| w.load(Ordering::Relaxed)).collect();
    let start = Instant::now();
    let mut prev_start = start;
    let mut total_hashes = 0;
    loop {
        println!("worker stats (since last):");
        let now = Instant::now();
        let cur_dur = now - prev_start;
        let total_dur = now - start;
        prev_start = now;
        let mut cur_hashes = 0;
        for (i, (prev, new)) in prevstats.iter_mut().zip(&workerstats).enumerate() {
            let new = new.load(Ordering::Relaxed);
            let cur = new - *prev;
            println!("\t{}: {} H/s", i, (cur as f32) / dur_to_f32(&cur_dur));
            cur_hashes += cur;
            *prev = new;
        }
        total_hashes += cur_hashes;
        println!("\ttotal (since last): {} H/s", (cur_hashes as f32) / dur_to_f32(&cur_dur));
        println!("\ttotal (all time): {} H/s", (total_hashes as f32) / dur_to_f32(&total_dur));
        std::io::stdin().read_line(&mut String::new()).unwrap();
    }
}

fn dur_to_f32(dur: &Duration) -> f32 {
    ((dur.as_secs() as f32) + (dur.subsec_nanos() as f32) / 1_000_000_000.0)
}

#[cfg(test)]
mod tests {}

pub struct Client {
    job: Arc<Mutex<Job>>,
}

impl Client {
    fn new(job: Job) -> Self {
        let job = Arc::new(Mutex::new(job));
        Client { job }
    }

    fn job_handle(&self) -> Arc<Mutex<Job>> {
        Arc::clone(&self.job)
    }
}

impl MessageHandler for Client {
    fn job_command(&mut self, j: Job) {
        debug!("new job: {:?}", j);
        *self.job.lock().unwrap() = j;
    }

    fn error_reply(&mut self, _id: RequestId, error: ErrorReply) {
        warn!(
            "received error: {:?}, assuming that indicates a stale share",
            error
        );
    }

    fn status_reply(&mut self, _id: RequestId, status: String) {
        if status == "OK" {
            debug!("received status OK");
        } else {
            info!("received status {:?}, assuming that means OK", status);
        }
    }

    fn job_reply(&mut self, _id: RequestId, _job: Box<JobAssignment>) {
        warn!("unexpected job reply...");
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerConfig {
    cpu: u32,
    hasher: cryptonight::HasherConfig,
}

pub fn run_worker(cfg: WorkerConfig, mut worksource: WorkSource, hash_count: Arc<AtomicUsize>, core: CoreId, worker_id: u32, step: u32) -> ! {
    // TODO: CoreId error handling
    core_affinity::set_for_current(core);
    let (mut target, blob, mut algo) = worksource.get_new_work().unwrap();
    let start = (u32::from(blob[42]) << 24) + worker_id;
    let nonce_seq = (start..).step_by(step as usize);
    let mut hashes = cryptonight::hasher(&algo, &cfg.hasher, blob, nonce_seq.clone());
    loop {
        let mut nonces = nonce_seq.clone();
        loop {
            let ws = &mut worksource;
            let mut h = [0u8; 32];
            h.copy_from_slice(&hashes.by_ref().next().unwrap());
            let n = nonces.by_ref().next().unwrap();
            if LE::read_u64(&h[24..]) <= target {
                ws.submit(&algo, n, &h).unwrap();
            }
            hash_count.fetch_add(1, Ordering::Relaxed);
            if let Some((newt, newb, newa)) = ws.get_new_work() {
                hashes = cryptonight::hasher(&newa, &cfg.hasher, newb, nonce_seq.clone());
                target = newt;
                algo = newa;
                break;
            }
        }
    }
}

