extern crate num_cpus;
extern crate yaml_rust;
use ::std::net::{SocketAddr};
use ::std::fs::File;
use ::std::io::{Read, Write};

pub struct WorkerConfig {
    pub server_addr:        SocketAddr,
    pub total_threads:      usize,
}

impl WorkerConfig {
    fn new(s: &String) -> Self {
        use self::yaml_rust::{Yaml, YamlLoader};
        let y = YamlLoader::load_from_str(s).unwrap();
        if y.len() > 1 {
            eprintln!("Ignoring multiple YAMLs");
        }
        let y = &y[0];

        let mut wc = WorkerConfig {
            server_addr:        "0.0.0.0:0".parse().unwrap(),
            total_threads:      0,
        };
        
        match y {
            &Yaml::Hash(ref hash)  => {
                let max_threads = self::num_cpus::get() as i64;
                let threads = match hash.get(&Yaml::String("threads".into())) {
                    Some(s) => match s {
                        &Yaml::Integer(ref i) => match *i {
                            0 => max_threads,
                            i if i > max_threads => {
                                eprintln!("Using {} threads instead of {}", max_threads, i);
                                max_threads
                            },
                            i if i > -max_threads && i < 0 => max_threads - i,
                            i if i <= -max_threads => {
                                eprintln!("Using a single thread.");
                                1
                            },
                            i => i,
                        },
                        &Yaml::String(ref s) => {
                            if s == "all" {
                                max_threads
                            } else {
                                panic!("threads value should be a non quoted integer or \"all\".");
                            }
                        },
                        _ => { panic!("threads value should be a non quoted integer or \"all\"."); },
                    },
                    None    => max_threads,
                };
                wc.total_threads = threads as usize;
                match hash.get(&Yaml::String("server".into())) {
                    Some(s) => match s {
                        &Yaml::String(ref s) => {
                            wc.server_addr = s.parse().unwrap();
                        },
                        _ => { panic!("server value should be a quoted string."); },
                    },
                    None    => { panic!("server expected."); },
                }
            },
            _ => { panic!("Hash expected."); },
        }

        wc
    }
}

fn get_default_string() -> String {
    let mut s = String::new();
    s += "server: \"127.0.0.1:7777\"\n";
    s += "threads: \"all\"\n";
    s
}

pub fn load_config() -> WorkerConfig {
    let mut path = match ::std::env::home_dir() {
        Some(home) => home,
        None       => {
            unimplemented!();
        },
    };
    path.push(".pitanga");
    path.push("worker.yml");

    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(_) => {
            let mut file = File::create(&path).unwrap();
            let _ = file.write(get_default_string().as_bytes()).unwrap();
            panic!("\nCould not read config from {}. Creating default config. Edit it.\n", path.to_str().unwrap());
        }
    };

    let mut s = String::new();
    let _ = file
        .read_to_string(&mut s)
        .unwrap();

    WorkerConfig::new(&s)
}
