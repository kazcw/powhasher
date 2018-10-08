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

mod stats;
mod worksource;

use std::fs::File;
use std::mem;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cn_stratum::client::{PoolClient, MessageHandler, RequestId, Job, JobAssignment, ErrorReply};
use crate::worksource::WorkSource;

use log::*;
use serde_derive::{Deserialize, Serialize};

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
            let (stat_updater, stat_reader) = stats::channel();
            let ws = worksource.clone();
            let core = core_ids[w.cpu as usize];
            debug!("starting worker{} with config: {:?}", i, &w);
            thread::Builder::new()
                .name(format!("worker{}", i))
                .spawn(move || run_worker(w, ws, stat_updater, core, i as u32, worker_count as u32))
                .unwrap();
            stat_reader
        })
        .collect();

    let mut prev_stats: Vec<_> = workerstats.iter().map(|w| w.get()).collect();
    let mut new_stats = Vec::new();
    loop {
        println!("worker stats (since last):");
        let mut cur_hashes = 0;
        let mut cur_dur = Duration::new(0, 0);
        let mut total_hashes = 0;
        let mut total_dur = Duration::new(0, 0);
        new_stats.clear();
        new_stats.extend(workerstats.iter().map(|w| w.get()));
        for (i, (new, old)) in new_stats.iter().zip(&prev_stats).enumerate() {
            let hashes = new.hashes - old.hashes;
            let runtime = new.runtime.checked_sub(old.runtime).unwrap();
            let rate = (hashes as f32)
                / ((runtime.as_secs() as f32) + (runtime.subsec_nanos() as f32) / 1_000_000_000.0);
            println!("\t{}: {} H/s", i, rate);
            cur_hashes += hashes;
            cur_dur = cur_dur.checked_add(runtime).unwrap();
            total_hashes += new.hashes;
            total_dur = total_dur.checked_add(new.runtime).unwrap();
        }
        let cur_rate = ((workerstats.len() * cur_hashes) as f32)
            / ((cur_dur.as_secs() as f32) + (cur_dur.subsec_nanos() as f32) / 1_000_000_000.0);
        println!("\ttotal (since last): {} H/s", cur_rate);
        let total_rate = ((workerstats.len() * total_hashes) as f32)
            / ((total_dur.as_secs() as f32) + (total_dur.subsec_nanos() as f32) / 1_000_000_000.0);
        println!("\ttotal (all time): {} H/s", total_rate);
        mem::swap(&mut prev_stats, &mut new_stats);

        std::io::stdin().read_line(&mut String::new()).unwrap();
    }
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

use crate::stats::StatUpdater;
use core_affinity::CoreId;
use byteorder::{ByteOrder, LE};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerConfig {
    cpu: u32,
    hasher: cryptonight::HasherConfig,
}

pub fn run_worker(cfg: WorkerConfig, mut worksource: WorkSource, mut stat_updater: StatUpdater, core: CoreId, worker_id: u32, step: u32) -> ! {
    // TODO: CoreId error handling
    core_affinity::set_for_current(core);
    stat_updater.reset();
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
            stat_updater.log_hashes(1);
            if let Some((newt, newb, newa)) = ws.get_new_work() {
                hashes = cryptonight::hasher(&newa, &cfg.hasher, newb, nonce_seq.clone());
                target = newt;
                algo = newa;
                break;
            }
        }
    }
}

