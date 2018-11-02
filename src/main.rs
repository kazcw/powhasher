// copyright 2017 Kaz Wesley

#![feature(alloc_system)]

// no allocs on hot paths anyway
extern crate alloc_system;

use std::fs::File;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use cn_stratum::client::{
    ErrorReply, Job, JobAssignment, MessageHandler, PoolClient, PoolClientWriter, RequestId,
};
use yellowsun::{Algo, AllocPolicy, Hasher};

use byteorder::{ByteOrder, LE};
use core_affinity::CoreId;
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
    pub cores: Vec<u32>,
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
                .required(true)
                .takes_value(true),
        ).arg(
            clap::Arg::with_name("allow-slow-mem")
                .long("allow-slow-mem")
                .help("Continue even if hugepages are not available (SLOW!)"),
        ).get_matches();

    let cfg: Config = File::open(args.value_of("config").unwrap())
        .map(serde_json::from_reader)
        .unwrap()
        .unwrap();
    debug!("config: {:?}", &cfg);

    let alloc_policy = if args.is_present("allow-slow-mem") {
        warn!("Slow memory enabled! Performance may be poor.");
        AllocPolicy::AllowSlow
    } else {
        AllocPolicy::RequireFast
    };

    let client = PoolClient::connect(
        &cfg.pool.address,
        &cfg.pool.login,
        &cfg.pool.pass,
        cfg.pool.keepalive_s.map(Duration::from_secs),
        AGENT,
        Client::new,
    ).unwrap();
    let work = client.handler().work();
    let pool = client.write_handle();
    thread::Builder::new()
        .name("poolclient".into())
        .spawn(move || client.run())
        .unwrap();

    let core_ids = core_affinity::get_core_ids().unwrap();
    let worker_count = cfg.cores.len();
    let mut workerstats = Vec::with_capacity(cfg.cores.len());
    for (i, w) in cfg.cores.into_iter().enumerate() {
        let hash_count = Arc::new(AtomicUsize::new(0));
        workerstats.push(Arc::clone(&hash_count));
        let core = core_ids[w as usize];
        debug!("starting worker{} on core {:?}", i, w);
        let worker = Worker {
            hash_count,
            work: Arc::clone(&work),
            pool: Arc::clone(&pool),
            core,
            worker_id: i as u32,
            step: worker_count as u32,
            alloc_policy,
        };
        thread::Builder::new()
            .name(format!("worker{}", i))
            .spawn(move || worker.run())
            .unwrap();
    }

    let mut prevstats: Vec<_> = workerstats
        .iter()
        .map(|w| w.load(Ordering::Relaxed))
        .collect();
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
        println!(
            "\ttotal (since last): {} H/s",
            (cur_hashes as f32) / dur_to_f32(&cur_dur)
        );
        println!(
            "\ttotal (all time): {} H/s",
            (total_hashes as f32) / dur_to_f32(&total_dur)
        );
        std::io::stdin().read_line(&mut String::new()).unwrap();
    }
}

fn dur_to_f32(dur: &Duration) -> f32 {
    ((dur.as_secs() as f32) + (dur.subsec_nanos() as f32) / 1_000_000_000.0)
}

pub struct Client {
    work: Arc<Work>,
}

impl Client {
    fn new(job: Job) -> Self {
        let work = Arc::new(Work::new(job));
        Client { work }
    }

    fn work(&self) -> Arc<Work> {
        Arc::clone(&self.work)
    }
}

impl MessageHandler for Client {
    fn job_command(&mut self, j: Job) {
        debug!("new job: {:?}", j);
        self.work.set_current(j);
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

#[derive(PartialEq, Eq, Copy, Clone)]
pub struct JobId(usize);
pub struct Work {
    job_id: AtomicUsize,
    job: Mutex<Job>,
}
impl Work {
    pub fn new(job: Job) -> Self {
        let job_id = AtomicUsize::new(0);
        let job = Mutex::new(job);
        Work { job_id, job }
    }
    pub fn is_current(&self, jid: JobId) -> bool {
        jid == JobId(self.job_id.load(Ordering::Relaxed))
    }
    pub fn current(&self) -> (JobId, Job) {
        (
            JobId(self.job_id.load(Ordering::Acquire)),
            self.job.lock().unwrap().clone(),
        )
    }
    pub fn set_current(&self, j: Job) {
        *self.job.lock().unwrap() = j;
        self.job_id.fetch_add(1, Ordering::Release);
    }
}

struct Worker {
    hash_count: Arc<AtomicUsize>,
    work: Arc<Work>,
    pool: Arc<Mutex<PoolClientWriter>>,
    core: CoreId,
    worker_id: u32,
    step: u32,
    alloc_policy: AllocPolicy,
}

const DEFAULT_ALGO: Algo = Algo::Cn2;

impl Worker {
    fn run(self) -> ! {
        core_affinity::set_for_current(self.core);
        let mut algo = DEFAULT_ALGO;
        loop {
            let mut hasher = Hasher::new(algo, self.alloc_policy);
            algo = loop {
                trace!("getting work");
                let (jid, job) = self.work.current();
                let new_algo = job
                    .algo()
                    .map(|x| x.parse().unwrap())
                    .unwrap_or_else(|| DEFAULT_ALGO);
                if new_algo != algo {
                    debug!("new algo: {:?}", new_algo);
                    break new_algo;
                }
                trace!("same algo ({:?})", new_algo);
                let start = (u32::from(job.blob()[42]) << 24) + self.worker_id;
                let nonce_seq = (start..).step_by(self.step as usize);
                let hashes = hasher.hashes(job.blob().into(), nonce_seq.clone());
                for (h, n) in hashes.zip(nonce_seq.clone()) {
                    if LE::read_u64(&h[24..]) <= job.target() {
                        debug!("submitting share");
                        self.pool.lock().unwrap().submit(&job, n, &h).unwrap();
                    }
                    self.hash_count.fetch_add(1, Ordering::Relaxed);
                    if !self.work.is_current(jid) {
                        trace!("work is outdated");
                        break;
                    }
                }
            }
        }
    }
}
