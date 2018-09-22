// copyright 2017 Kaz Wesley

// core_affinity (maintained)
// hwloc (maintained)
// serde+serde_json

// libcpuid (rs wrapper broken, needs work/replacement)
// memmap (no hugepage support)

#![feature(exact_chunks)]
#![feature(try_from)]
#![feature(type_ascription)]

mod poolclient;
mod worker;

use std::fs::File;
use std::mem;
use std::thread;
use std::time::Duration;

use crate::worker::stats;
use crate::worker::Worker;

use log::{debug, log};
use serde_derive::{Deserialize, Serialize};

const AGENT: &str = "pow#er/0.2.0";

#[derive(Deserialize, Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
    pub pool: poolclient::Config,
    pub workers: Vec<worker::Config>,
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

    let worksource = poolclient::run_thread(&cfg.pool, AGENT).unwrap();

    let core_ids = core_affinity::get_core_ids().unwrap();
    let worker_count = cfg.workers.len();
    let workerstats: Vec<_> = cfg.workers
        .into_iter()
        .enumerate()
        .map(|(i, w)| {
            let (stat_updater, stat_reader) = stats::channel();
            let worker = Worker::new(worksource.clone(), stat_updater);
            let core_ids = core_ids.clone();
            debug!("starting worker{} with config: {:?}", i, &w);
            thread::Builder::new()
                .name(format!("worker{}", i))
                .spawn(move || worker.run(w, core_ids, i as u32, worker_count as u32))
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
