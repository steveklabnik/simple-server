use super::Request;
use error::Error;
use std::io::{self, Read};
use std::time::{Duration, Instant};

use parsing;

fn elapsed_milliseconds(from: &Instant) -> u64 {
    let elapsed = Instant::now() - *from;
    (elapsed.as_secs() * 1000) + (elapsed.subsec_nanos() as u64 / 1_000_000)
}

fn duration_to_milliseconds(from: &Duration) -> u64 {
    (from.as_secs() * 1000) + (from.subsec_nanos() as u64 / 1_000_000)
}

pub fn read<S: Read>(stream: &mut S, timeout: Option<Duration>) -> Result<Request<Vec<u8>>, Error> {
    use std::mem;

    let start_time = Instant::now();
    let mut buffer = Vec::with_capacity(512);
    let mut read_buf = [0_u8; 512];

    let request = loop {
        match stream.read(&mut read_buf) {
            Ok(0) => return Err(Error::ConnectionClosed),
            Ok(n) => {
                buffer.extend_from_slice(&read_buf[..n]);
                match parsing::try_parse_request(mem::replace(&mut buffer, vec![]))? {
                    parsing::ParseResult::Complete(r) => break r,
                    parsing::ParseResult::Partial(b) => {
                        mem::replace(&mut buffer, b);
                        continue;
                    }
                }
            }
            Err(e) => {
                if e.kind() != io::ErrorKind::WouldBlock && e.kind() != io::ErrorKind::TimedOut {
                    return Err(e.into());
                }

                if timeout.is_some()
                    && elapsed_milliseconds(&start_time)
                        > duration_to_milliseconds(&timeout.unwrap())
                {
                    return Err(Error::Timeout);
                }

                continue;
            }
        }
    };

    build_request(request)
}

fn build_request(mut req: parsing::Request) -> Result<Request<Vec<u8>>, Error> {
    let mut http_req = Request::builder();

    http_req.method(req.method());

    for header in req.headers() {
        http_req.header(header.name, header.value);
    }

    let mut request = http_req.body(req.split_body())?;
    let path = req.path();
    *request.uri_mut() = path.parse()?;

    Ok(request)
}

#[cfg(test)]
mod server_should {

    use super::*;
    use http::method::Method;

    static HTTP_REQUEST: &'static [u8] = include_bytes!("../tests/big-http-request.txt");
    static PUT_REQUEST: &'static [u8] = b"PUT / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";

    struct ChunkStream<'content> {
        content: &'content [u8],
        bytes_read: usize,
        read_count: usize,
        timeout: Option<Duration>,
    }

    impl<'content> ChunkStream<'content> {
        fn new(content: &'content [u8]) -> ChunkStream<'content> {
            ChunkStream {
                content: content,
                bytes_read: 0,
                read_count: 0,
                timeout: None,
            }
        }

        fn with_timeout(content: &'content [u8], timeout: Duration) -> ChunkStream<'content> {
            ChunkStream {
                content: content,
                bytes_read: 0,
                read_count: 0,
                timeout: Some(timeout),
            }
        }
    }

    impl<'content> Read for ChunkStream<'content> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            use std::thread;

            if let Some(timeout) = self.timeout {
                thread::sleep(timeout);
                Err(io::Error::new(io::ErrorKind::TimedOut, ""))
            } else {
                let read = match self.read_count {
                    0 => {
                        let half = self.content.len() / 2;
                        let min = ::std::cmp::min(half, buf.len());
                        &buf[..min].copy_from_slice(&self.content[..min]);
                        min
                    }
                    _ => {
                        let min = ::std::cmp::min(self.content[self.bytes_read..].len(), buf.len());
                        &buf[..min]
                            .copy_from_slice(&self.content[self.bytes_read..self.bytes_read + min]);
                        min
                    }
                };

                self.bytes_read += read as usize;
                self.read_count += 1;

                Ok(read as usize)
            }
        }
    }

    #[test]
    fn read_request_stream_in_multiple_chunks() {
        let mut s = ChunkStream::new(HTTP_REQUEST);

        assert!(read(&mut s, None).is_ok());
    }

    #[test]
    fn honour_request_timeout() {
        let timeout = Duration::from_millis(50);
        let mut s = ChunkStream::with_timeout(HTTP_REQUEST, timeout);

        let result = read(&mut s, Some(timeout));

        match result {
            Err(Error::Timeout) => {}
            Err(e) => panic!("Expected timeout but got {:?}", e),
            Ok(_) => panic!("Expected timeout error but got Ok(_)"),
        }
    }

    #[test]
    fn correctly_parse_request() {
        use http::header::*;
        let mut s = ChunkStream::new(HTTP_REQUEST);
        let r = read(&mut s, None).unwrap();
        assert_eq!(4, r.headers().len());
        assert_eq!("127.0.0.1", r.headers()[HOST]);
        assert!(r.headers().contains_key("X-SOME-HEADER"));
        assert!(r.headers().contains_key("X-SOMEOTHER-HEADER"));
        assert!(r.headers().contains_key("X-ONEMORE-HEADER"));
    }

    #[test]
    fn parse_method_correctly() {
        let mut s = ChunkStream::new(PUT_REQUEST);
        let req = read(&mut s, None).expect("Failed to parse PUT request.");
        assert_eq!(Method::PUT, *req.method());
    }
}
