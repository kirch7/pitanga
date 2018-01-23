extern crate yaml_rust;

use self::yaml_rust::{Yaml, YamlLoader};
use ::std::net::{IpAddr, AddrParseError};
use ::std::process::Child;
use ::std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct Job {
    id: usize,          // Set on server.
    pub pid: usize,     // Set on worker.
    ip: Option<IpAddr>, // Set on server.
    threads: usize,     // Set on server.
    script: String,     // Set on server.
    pub child: Arc<Mutex<Option<Child>>>,
}

#[derive(Debug)]
pub enum Error {
    NotHash(Yaml),
    NotString(Yaml),
    NotInteger(Yaml),
    DuplicatedTag(String),
    UndefinedTag(String),
    AddrParse(AddrParseError),
}

impl Job {
    pub fn script(&self) -> &String {
        &self.script
    }
    pub fn threads(&self) -> usize {
        self.threads
    }
    pub fn id(&self) -> usize {
        self.id
    }
    pub fn ip(&self) -> Option<IpAddr> {
        self.ip
    }
}

impl ::std::string::ToString for Job {
    fn to_string(&self) -> String {
        self.script.clone()
    }
}

impl ::std::str::FromStr for Job {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut job = Job {
            id:      0,
            pid:     0,
            ip:      None,
            threads: 0,
            script:  s.to_string(),
            child:   Arc::default(),
        };
        
        const COMMENT: &str = "#PITANGA";
        let comment_len = COMMENT.len();

        let mut yaml = String::new();
        for line in s.lines() {
            if line.starts_with(COMMENT) {
                let mut line = line.to_string();
                for _ in 0..comment_len {
                    let _ = line.remove(0);
                }
                yaml += &line;
                yaml += "\n";
            }
        }
        
        let yaml = YamlLoader::load_from_str(&yaml).unwrap();
        if yaml.len() > 1 {
            eprintln!("Ignoring several YAMLs");
        }
        let yaml = &yaml[0];
        if let &Yaml::Hash(ref hash) = yaml {
            let mut hash = hash.clone();
            let mut threads_defined = false;
            let mut pid_defined = false;
            let mut id_defined  = false;
            let mut ip_defined  = false;
            for pair in hash.entries() {
                let key = pair.key();
                let value = pair.get();
                match key {
                    &Yaml::String(ref key) => {
                        match key.as_str() {
                            "ID" => {
                                if id_defined {
                                    use self::Error::DuplicatedTag;
                                    return Err(DuplicatedTag("#PITANGAID".into()));
                                }
                                id_defined = true;
                                if let &Yaml::Integer(id) = value {
                                    job.id = id as usize;
                                } else {
                                    use self::Error::NotInteger;
                                    return Err(NotInteger(value.clone()));
                                }
                            },
                            "PID" => {
                                if pid_defined {
                                    use self::Error::DuplicatedTag;
                                    return Err(DuplicatedTag("#PITANGAPID".into()));
                                }
                                pid_defined = true;
                                if let &Yaml::Integer(pid) = value {
                                    job.pid = pid as usize;
                                } else {
                                    use self::Error::NotInteger;
                                    return Err(NotInteger(value.clone()));
                                }
                            },
                            "IP" => {
                                if ip_defined {
                                    use self::Error::DuplicatedTag;
                                    return Err(DuplicatedTag("#PITANGAIP".into()));
                                }
                                ip_defined = true;
                                if let &Yaml::String(ref ip) = value {
                                    let ip = ip.parse();
                                    match ip {
                                        Ok(ip) => {
                                            job.ip = Some(ip);
                                        },
                                        Err(e) => {
                                            use self::Error::AddrParse;
                                            return Err(AddrParse(e));
                                        },
                                    }
                                } else {
                                    use self::Error::NotString;
                                    return Err(NotString(value.clone()));
                                }
                            },
                            "THREADS" => {
                                if threads_defined {
                                    use self::Error::DuplicatedTag;
                                    return Err(DuplicatedTag("#PITANGATHREADS".into()));
                                }
                                threads_defined = true;
                                if let &Yaml::Integer(threads) = value {
                                    job.threads = threads as usize;
                                } else {
                                    use self::Error::NotInteger;
                                    return Err(NotInteger(value.clone()));
                                }
                            },
                            other => {
                                eprintln!("{} received", other);
                            },
                        }
                    }
                    not_string => {
                        use self::Error::NotString;
                        return Err(NotString(not_string.clone()));
                    },
                }
                
            }
            if !threads_defined {
                use self::Error::UndefinedTag;
                return Err(UndefinedTag("#PITANGATHREADS".into()));
            }
        } else {
            use self::Error::NotHash;
            return Err(NotHash(yaml.clone()));
        }
        Ok(job)
    }
}

