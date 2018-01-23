extern crate libpitanga;

use libpitanga::message;
use std::net::{TcpStream, SocketAddr};
use std::thread;
use std::time;
use std::fs::File;
use std::io::Read;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    
    let mut script = String::new();
    let mut file = File::open(&args[1]).unwrap();
    file.read_to_string(&mut script).unwrap();
    script += "\n";
    
    let mut stream = get_stream("127.0.0.1:7777".parse().unwrap());
    let order = message::receive(&mut stream).unwrap();
    match order.as_str() {
        "ENTRYPOINT" => {
            message::send("BATCH".into(), &mut stream).unwrap();
            let order = message::receive(&mut stream).unwrap();
            match order.as_str() {
                "THENSENDMETHESCRIPT" => {
                    message::send(script, &mut stream).unwrap();
                    let status = message::receive(&mut stream).unwrap();
                    println!("{}", status);
                    // TODO: handle statue.
                    message::send("".into(), &mut stream).unwrap();
                },
                other => { panic!("THENSENDMETHESCRIPT expected. {} received.", other);; },
            }
        },
        other => { panic!("ENTRYPOINT expected. {} received.", other);; },
    }
}

fn get_stream(addr: SocketAddr) -> TcpStream {
    loop {
        match TcpStream::connect(addr) {
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
