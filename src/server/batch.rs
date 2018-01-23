use ::WorkerStatusHash;
use ::libpitanga::job::Job;
use ::std::str::FromStr;
use ::std::path::PathBuf;
use ::std::fs::{self, File};
use ::std::io::{Read, Write};
use ::std::{time, thread};
use ::std::net::IpAddr;

pub fn get_batch_thread(status: WorkerStatusHash) -> Result<thread::JoinHandle<()>, ::std::io::Error> {
    let thread = thread::Builder::new()
        .name("batch".into());
    let thread = thread.spawn(move || {
        let status = status.clone();
        loop {
            let queued_paths = ::get_suffixed_jobs("_queued.pitanga");
            println!("batch thread: {:?}", queued_paths);
            if queued_paths.len() == 0 {
                thread::sleep(time::Duration::from_millis(1456));
                continue;
            }

            let mut some_assigned = false;
            for path in &queued_paths {
                let assigned = assign(path, status.clone()).is_some();
                some_assigned = some_assigned || assigned;
            }
            if !some_assigned {
                thread::sleep(time::Duration::from_millis(1345));
                continue;
            }
        }
    });
    thread
}

fn assign(path: &PathBuf, hash: WorkerStatusHash) -> Option<()> {
    // TODO: remove unwraps and handle errors.
    let mut file = File::open(path).unwrap();
    let mut job = String::new();
    file.read_to_string(&mut job).unwrap();
    let job = Job::from_str(&job).unwrap();

    let hash = hash.lock().unwrap();
    for (ip, arc_tuple) in &*hash {
        ////println!("assign: {:?}", worker_status);
        let &(ref worker_status, ref cv) = &**arc_tuple;
        let mut worker_status = worker_status.lock().unwrap();
        if worker_status.next_job.is_some() {
            println!("assign: already assigned {:?}", worker_status.next_job);
            continue;
        }
        if worker_status.available_threads >= job.threads() {
            worker_status.available_threads -= job.threads();
            let script = format!("#PITANGAIP: \"{}\"\n", ip) + &job.script();
            ::std::mem::forget(file);
            fs::remove_file(path).unwrap();
            let mut path = path.parent().unwrap();
            let path = path.join(format!("{}_running.pitanga", job.id()));
            let mut file = File::create(path).unwrap();
            file.write_all(script.as_bytes()).unwrap();
            let job = Job::from_str(&script).unwrap();
            worker_status.next_job = Some(job.clone());
            worker_status.running_jobs.push(job);
            cv.notify_all();
            return Some(());
        }
    }
    
    None
}

pub fn submit_job(job: &String) -> String {
    let job_id = ::get_last_job_id() + 1;
    let job = format!("#PITANGAID: {}\n", job_id) + &job;

    let mut path = ::get_pitanga_path();
    path.push("jobs");
    path.push(format!("{}_queued.pitanga", job_id));
    
    let mut file = File::create(path).unwrap();
    file.write(job.as_bytes()).unwrap();
    format!("Job {} queued.", job_id)
}

pub fn requeue_all_jobs_from_a_worker(ip: &IpAddr) {
    let path = ::get_pitanga_path()
        .join("jobs");
    let old_suffix = "_running.pitanga";
    let new_suffix = "_queued.pitanga";
    ::get_suffixed_jobs(old_suffix)
        .iter()
        .map(|path| File::open(path).unwrap())
        .map(|mut file| {
            let mut s = String::default();
            file.read_to_string(&mut s).unwrap();
            s
        })
        .map(|string| Job::from_str(&string).unwrap())
        .filter(|job| job.ip().is_some())
        .filter(|job| &job.ip().unwrap() == ip)
        .map(|job| job.script().replace("#PITANGAIP", "#PITANGAOLDIP"))
        .map(|string| Job::from_str(&string).unwrap())
        .for_each(|job| {
            let mut file = File::create(path.join(format!("{}{}", job.id(), new_suffix))).unwrap();
            file.write(job.script().as_bytes()).unwrap();
            fs::remove_file(path.join(format!("{}{}", job.id(), old_suffix))).unwrap();
        });
}
