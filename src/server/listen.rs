use ::{CvPair, WorkerStatusHash};
use ::batch;
use ::libpitanga::message;
use ::std::net::{TcpStream, TcpListener, IpAddr};
use ::std::thread;
use ::std::time;
use ::std::sync::{Arc, Mutex, Condvar, mpsc};

mod handle {
    use ::libpitanga::message;
    use ::std::net::TcpStream;
    use ::std::sync::mpsc::Sender;
    pub fn send(s: String, stream: &mut TcpStream, sender: &Sender<()>) -> Result<(), message::Error> {
        let result = message::send(s, stream);
        if result.is_err() {
            sender.send(()).unwrap();
            println!("Error: {:?}", result);
        }
        result
    }

    pub fn receive(stream: &mut TcpStream, sender: &Sender<()>) -> Result<String, message::Error> {
        let received = message::receive(stream);
        if received.is_err() {
            sender.send(()).unwrap();
            println!("Error: {:?}", received);
        }
        received
    }
}

fn listen(stream: &mut TcpStream, cv_pair: CvPair, sender: mpsc::Sender<()>) {
    loop {
        let cv_pair = cv_pair.clone();
        let &(ref status, ref cv) = &*cv_pair;
        let mut status = status.lock().unwrap();
        if handle::send("NEWFINISHEDJOBS".into(), stream, &sender).is_err() {
            return;
        }
        match handle::receive(stream, &sender) {
            Ok(ref received) => received
                .split(" ")
                .filter(|string| !string.is_empty())
                .map(|string| string.parse::<usize>().unwrap())
                .for_each(|job_id| ::finish_job(job_id).unwrap()),
            Err(_) => { return; },
        }
        
        if handle::send("AVAILABLETHREADS".into(), stream, &sender).is_err() {
            return;
        }
        status.available_threads = match handle::receive(stream, &sender) {
            Ok(no) => no
                .parse()
                .unwrap(),
            Err(_) => { return; },
        };
        let mut status = match cv.wait_timeout(status, time::Duration::from_secs(5)) {
            Ok(tuple) => tuple.0,
            Err(_)          => { continue; },
        };
        let job = status.next_job.clone();
        let mut job = match job {
            Some(ref job) => {
                let job = job.clone();
                status.next_job = None;
                job
            },
            None          => {
                thread::sleep(time::Duration::from_millis(1024));
                continue;
            },
        };
        if handle::send("RUN\n".to_string() + &job.to_string(), stream, &sender).is_err() {
            return;
        }
        let pid = match handle::receive(stream, &sender) {
            Ok(pid) => pid,
            Err(_)  => { return; },
        };
        job.pid = pid.parse().unwrap();
    }
}

pub fn get_listener_threads(listener: TcpListener, worker_status_hash: WorkerStatusHash) -> (Result<thread::JoinHandle<()>, ::std::io::Error>, Result<thread::JoinHandle<()>, ::std::io::Error>) {
    use ::libpitanga::worker_status::ThreadStatus;

    let worker_status_hash_killer_clone   = worker_status_hash.clone();
    let worker_status_hash_listener_clone = worker_status_hash.clone();

    let worker_threads = Arc::new(Mutex::new(::HashMap::<IpAddr, (thread::JoinHandle<()>, mpsc::Receiver<()>)>::default()));
    let worker_threads_killer_clone   = worker_threads.clone();
    let worker_threads_listener_clone = worker_threads.clone();
    
    let killer_thread = thread::Builder::new()
        .name("killer".into());
    let killer_thread = killer_thread
        .spawn(move || {
            loop {
                thread::sleep(time::Duration::from_millis(963));
                let mut worker_status_hash = worker_status_hash_killer_clone.lock().unwrap();
                let worker_threads = worker_threads_killer_clone.lock().unwrap();
                for (ip, &(ref _thread, ref receiver)) in &*worker_threads {
                    if receiver.try_recv().is_ok() {
                        let _ = worker_status_hash.remove(ip).unwrap();
                    }
                }
            }
        });

    let listener_thread = thread::Builder::new()
        .name("listener".into());
    let listener_thread = listener_thread
        .spawn(move || {
            loop {
                let (mut stream, socket_addr) = match listener.accept() {
                    Ok(tuple) => tuple,
                    Err(err)  => {
                        eprintln!("Error: {}", err.to_string());
                        thread::sleep(time::Duration::from_millis(160));
                        continue;
                    },
                };
                println!("{:?}", stream);
                let ip = socket_addr.ip();
                let ip_str = format!("{}", ip);

                message::send("ENTRYPOINT".into(), &mut stream).unwrap();
                match message::receive(&mut stream).unwrap().as_str() {
                    "NEWWORKER" => {
                        let mut worker_status_hash = worker_status_hash_listener_clone.lock().unwrap();
                        let thread_status = match worker_status_hash.get(&ip) {
                            Some(ref cv_tuple) => {
                                let status = cv_tuple.0.lock().unwrap();
                                match &status.thread_status {
                                    &ThreadStatus::NeverStarted => {
                                        eprintln!("This should be impossible.");
                                        ThreadStatus::NeverStarted
                                    },
                                    thread_status => thread_status.clone(),
                                }
                            },
                            None => ThreadStatus::NeverStarted,
                        };

                        let start = |mut stream| {
                            let cv_pair = {
                                let mut worker_status = ::libpitanga::worker_status::WorkerStatus::default();
                                message::send("TOTALTHREADS".into(), &mut stream).unwrap();
                                worker_status.total_threads = message::receive(&mut stream)
                                    .unwrap()
                                    .parse()
                                    .unwrap();
                                worker_status.available_threads = worker_status.total_threads;
                                let cv_pair = CvPair::new((Mutex::new(worker_status), Condvar::new()));
                                worker_status_hash.insert(ip, cv_pair);
                                worker_status_hash.get(&ip).unwrap().clone()
                            };
                            let (sender, receiver) = mpsc::channel();
                            
                            let worker = thread::Builder::new()
                                .name(ip_str);
                            let worker = worker
                                .spawn(move || {
                                    listen(&mut stream, cv_pair, sender);
                                });
                            let mut worker_threads = worker_threads_listener_clone.lock().unwrap();
                            worker_threads.insert(ip, (worker.unwrap(), receiver));
                        };
                        
                        match thread_status {
                            ThreadStatus::NeverStarted => {
                                start(stream.try_clone().unwrap());
                            },
                            ThreadStatus::Running => {
                                eprintln!("Duplicated worker: {}", ip);
                                message::send("DENIED".into(), &mut stream).unwrap();
                            },
                            ThreadStatus::Stopped => {
                                batch::requeue_all_jobs_from_a_worker(&ip);
                                start(stream.try_clone().unwrap());
                            }
                        }
                    }, // End of WORKER.
                    "BATCH"  => {
                        message::send("THENSENDMETHESCRIPT".into(), &mut stream).unwrap();
                        let job = message::receive(&mut stream).unwrap();
                        let status = batch::submit_job(&job);
                        message::send(status, &mut stream).unwrap();
                        let _ = message::receive(&mut stream).unwrap();
                    },
                    other => eprintln!("Wrong message: {}", other),
                }
                
            } // End of loop.
        });
    (listener_thread, killer_thread)
}
