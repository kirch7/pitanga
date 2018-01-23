use job;

#[derive(Clone,  Debug)]
pub struct WorkerStatus {
    pub ip: Option<::std::net::IpAddr>,
    pub total_threads: usize,
    pub available_threads: usize,
    pub next_job: Option<job::Job>,
    pub running_jobs: Vec<job::Job>,
    //#[cfg(feature = "pitangaserver")]
    pub thread_status: ThreadStatus,
}

//#[cfg(feature = "pitangaserver")]
#[derive(Clone, Debug)]
pub enum ThreadStatus {
    NeverStarted,
    Running,
    Stopped,
}

impl Default for WorkerStatus {
    fn default() -> Self {
        WorkerStatus {
            ip: None,
            total_threads: 0,
            available_threads: 0,
            next_job: None,
            running_jobs: Vec::default(),
            //#[cfg(feature = "pitangaserver")]
            thread_status: ThreadStatus::NeverStarted,
        }
    }
}
