extern crate libpitanga;

use libpitanga::worker_status::WorkerStatus;
use libpitanga::path::get_pitanga_path;
use std::net::{IpAddr, TcpListener};
use std::sync::{Arc, Mutex, Condvar};
use std::path::PathBuf;
use std::collections::HashMap;
use std::fs;

type CvPair = Arc<(Mutex<WorkerStatus>, Condvar)>;
type WorkerStatusHash = Arc<Mutex<HashMap<IpAddr, CvPair>>>;

mod listen;
mod batch;

fn main() {
    let listener = match open_port(&"0.0.0.0:7777".to_string()) {
        Ok(l)  => l,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        },
    };
    let hash = WorkerStatusHash::default();
    let batch_thread     = batch::get_batch_thread(hash.clone()).unwrap();
    let listener_threads = listen::get_listener_threads(listener, hash.clone());

    let _ = listener_threads.0.unwrap().join().unwrap();
    let _ = listener_threads.1.unwrap().join().unwrap();
    let _ = batch_thread.join().unwrap();
}

fn open_port(addr: &String) -> Result<TcpListener, String> {
    match TcpListener::bind(addr) {
        Ok(o)  => Ok(o),
        Err(e) => Err(e.to_string()),
    }
}

fn get_last_job_id() -> usize {
    let mut path = get_pitanga_path();
    path.push("jobs");
    let mut vec = path
        .read_dir()
        .unwrap()
        .map(|r| r
             .unwrap()
             .path())
        .filter(|entry| entry
                .is_file())
        .map(|filename| filename
             .as_path()
             .file_stem()
             .unwrap()
             .to_str()
             .unwrap()
             .split("_")
             .nth(0)
             .map(|s| s.to_string()))
        .filter(|option| option.is_some())
        .map(|option| option.unwrap())
        .map(|string| string
             .parse::<usize>())
        .filter(|result| result.is_ok())
        .map(|result| result.unwrap())
        .collect::<Vec<_>>();
    vec.sort();
    if vec.len() == 0 {
        0
    } else {
        vec
            .iter()
            .last()
            .unwrap()
            .clone()
    }
}

fn get_suffixed_jobs(suffix: &str) -> Vec<PathBuf> {
    let mut path = get_pitanga_path();
    path.push("jobs");
    assert!(suffix.starts_with("_"));
    let mut vec = path
        .read_dir()
        .unwrap()
        .map(|r| r
             .unwrap()
             .path())
        .filter(|entry| entry
                .is_file())
        .map(|filename| filename
             .as_path()
             .file_name()
             .unwrap()
             .to_str()
             .unwrap()
             .to_string())
        .filter(|filename| filename
                .ends_with(suffix))
        .map(|filename| filename
             .split("_")
             .nth(0)
             .map(|s| s.to_string()))
        .filter(|option| option
                .is_some())
        .map(|option| option
             .unwrap()
             .parse())
        .filter(|result| result
                .is_ok())
        .map(|result| result
             .unwrap())             
        .collect::<Vec<usize>>();
    vec.sort();
    vec
        .iter()
        .map(|number| path.join(format!("{}{}", number, suffix)))
        .collect()
}

fn finish_job(id: usize) -> Result<(), String> {
    let base = get_pitanga_path().join("jobs");
    let from = base.join(format!("{}_running.pitanga", id));
    let to   = base.join(format!("{}_finished.pitanga", id));
    fs::rename(from, to).map_err(|e| e.to_string())
}
