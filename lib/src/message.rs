use ::std::io::{Read, Write};
use ::std::net::TcpStream;
use size::Size;

#[derive(Debug)]
pub enum Error {
    Io(::std::io::Error),
    Size(SizeErr),
    Utf8(::std::string::FromUtf8Error),
}

#[derive(Debug)]
pub struct SizeErr {
    was_transferred:       usize,
    should_be_transferred: usize,
}

fn receive_size(stream: &mut TcpStream) -> Result<usize, Error> {
    let mut size = Size::zero();
    let shall_receive = Size::bytes_no();

    match stream.read(size.mut_slice()) {
        Ok(received) => {
            if received == shall_receive {
                Ok(size.into())
            } else {
                Err(Error::Size(SizeErr {
                    was_transferred:       received,
                    should_be_transferred: shall_receive,
                }))
            }
        },
        Err(err)     => Err(Error::Io(err)),
    }
}

fn send_size(size: usize, stream: &mut TcpStream) -> Result<(), Error> {
    let size = Size::from(size);
    let shall_send = Size::bytes_no();

    match stream.write(size.slice()) {
        Ok(sent) => {
            if sent == shall_send {
                Ok(())
            } else {
                Err(Error::Size(SizeErr {
                    was_transferred:       sent,
                    should_be_transferred: shall_send,
                }))
            }
        },
        Err(err)     => Err(Error::Io(err)),
    }
}

pub fn receive(stream: &mut TcpStream) -> Result<String, Error> {
    match receive_size(stream)  {
        Ok(shall_receive) => {
            let mut v = vec![0u8; shall_receive];
            ::std::thread::sleep(::std::time::Duration::from_millis(100));
            match stream.read(v.as_mut_slice()) {
                Ok(received) => {
                    if received == shall_receive {
                        match String::from_utf8(v) {
                            Ok(string) => {
                                //println!("Received: {}", string);
                                Ok(string)
                            },
                            Err(err)   => Err(Error::Utf8(err)),
                        }
                    } else {
                        Err(Error::Size(SizeErr {
                            was_transferred:       received,
                            should_be_transferred: shall_receive,
                        }))
                    }
                },
                Err(err)     => Err(Error::Io(err)),
            }
        },
        Err(err)          => Err(err),        
    }
}

pub fn send(s: String, stream: &mut TcpStream) -> Result<(), Error> {
    //println!("Sending: {}", s);
    let vec = s.into_bytes();
    let vec_len = vec.len();
    match send_size(vec_len, stream) {
        Ok(()) => {
            match stream.write(vec.as_slice()) {
                Ok(sent) => {
                    if sent == vec_len {
                        Ok(())
                    } else {
                        Err(Error::Size(SizeErr {
                            was_transferred: sent,
                            should_be_transferred: vec_len,
                        }))
                    }
                },
                Err(err) => Err(Error::Io(err)),
            }
        },
        Err(err) => Err(err),
    }
}
