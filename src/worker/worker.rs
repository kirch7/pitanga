extern crate libpitanga;

use libpitanga::message;
use libpitanga::job::Job;
use std::str::FromStr;
use std::net::TcpStream;
use std::thread;
use std::time;
use std::sync::{Arc, Mutex};
use std::process::Command;

mod worker_config;

type WorkerConfig = Arc<Mutex<worker_config::WorkerConfig>>;
type WorkerStatus  = Arc<Mutex<libpitanga::worker_status::WorkerStatus>>;

fn main() {
    let config = worker_config::load_config();
    let config = Arc::new(Mutex::new(config));
    start(config);
}

fn get_stream(config: WorkerConfig) -> TcpStream {
    let config = config.lock().unwrap();
    loop {
        match TcpStream::connect(config.server_addr) {
            Ok(s)  => {
                println!("{:?}", s);
                return s;
            },
            Err(e) => {
                eprintln!("Error: {}", e.to_string());
                thread::sleep(time::Duration::from_millis(768));
                continue;
            },
        }
    }
}

fn start(config: WorkerConfig) {
    let mut stream = get_stream(config.clone());
    let order = message::receive(&mut stream).unwrap();
    let worker_status = WorkerStatus::default();
    {
        let mut worker_status = worker_status.lock().unwrap();
        let config = config.lock().unwrap();
        worker_status.total_threads = config.total_threads;
        worker_status.available_threads = config.total_threads;
    }
    match order.as_str() {
        "ENTRYPOINT" => {
            message::send("NEWWORKER".into(), &mut stream).unwrap();
        },
        other => { panic!("ENTRYPOINT expected. {} received.", other); },
    }
    loop {
        speak(&mut stream, worker_status.clone());
    }
}

fn speak(stream: &mut TcpStream, status: WorkerStatus) {
    let in_message = message::receive(stream).unwrap();
    match in_message.as_str() {
        "NEWFINISHEDJOBS" => {
            let mut ids_to_be_removed = Vec::default();
            let mut status = status.lock().unwrap();
            for job in &status.running_jobs {
                let mut child = &mut *job.child.lock().unwrap();
                match child {
                    &mut Some(ref mut child) => {
                        let child_status = &child.try_wait().unwrap();
                        if child_status.is_some() {
                            ids_to_be_removed.push((child.id(), job.id()));
                        }
                    },
                    _ => { },
                }
            }
            let mut threads_to_be_freed = 0;
            let mut answer = String::from_str(" ").unwrap();
            {
                let running_jobs = &mut status.running_jobs;
                'outer: loop {
                    let running_jobs_clone = running_jobs.clone();
                    for index in 0..running_jobs_clone.len() {
                        for &(pid, jid) in &ids_to_be_removed {
                            let job = running_jobs[index].clone();
                            if job.pid as u32 == pid {
                                answer += &format!("{} ", jid);
                                let _ = running_jobs.remove(index);
                                threads_to_be_freed += job.threads();
                                continue 'outer;
                            }
                        }
                    }
                    break;
                }
            }
            println!("{:?}", status.running_jobs);
            status.available_threads += threads_to_be_freed;
            println!("{}", answer);
            message::send(answer, stream).unwrap();
        }

        "AVAILABLETHREADS" => {
            let status = &status.lock().unwrap();
            message::send(status.available_threads.to_string(), stream).unwrap();
        },

        "TOTALTHREADS" => {
            let status = &status.lock().unwrap();
            message::send(status.total_threads.to_string(), stream).unwrap();
        },

        other => {
            if other.starts_with("RUN\n") {
                let mut script = other.to_string();
                for _ in 0..4 {
                    let _ = script.remove(0);
                }
                let mut job = Job::from_str(&script).unwrap();
                run_job(&mut job, status);
                message::send(job.pid.to_string(), stream).unwrap();
            } else {
                eprintln!("{} is not expected.", other);
            }
        },
    }
}

fn run_job(job: &mut Job, status: WorkerStatus) {
    let command = "/bin/bash";
    let child = Command::new(command)
        .arg("-c")
        .arg(job.script())
        .spawn()
        .unwrap();
    job.pid = child.id() as usize;
    {
        let child_option = Arc::get_mut(&mut job.child).unwrap();
        let mut child_option = child_option.lock().unwrap();
        *child_option = Some(child);
    }
    let mut status = status.lock().unwrap();
    status.available_threads -= job.threads();
    status.running_jobs.push(job.clone());
}
