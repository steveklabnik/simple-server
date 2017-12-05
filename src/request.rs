use std::io::{self, Read, Seek};
use std::time::{Duration, Instant};
use error::Error;
use super::Request;

use parsing;

fn elapsed_milliseconds(from: &Instant) -> u64 {
    let elapsed = Instant::now() - *from;
    (elapsed.as_secs() * 1000) + (elapsed.subsec_nanos() as u64 / 1_000_000)
}

fn duration_to_milliseconds(from: &Duration) -> u64 {
    (from.as_secs() * 1000) + (from.subsec_nanos() as u64 / 1_000_000)
}

pub fn read<S: Read>(stream: &mut S, timeout: Option<Duration>) -> Result<Request<Vec<u8>>, Error> {
    let start_time = Instant::now();
    let mut buffer = io::Cursor::new(Vec::with_capacity(512));

    let request = loop {

        buffer.seek(io::SeekFrom::End(0))?;
        match io::copy(stream, &mut buffer) {
            Ok(0) => return Err(Error::ConnectionClosed),
            Ok(_) => {
                match parsing::try_parse_request(buffer.into_inner())? {
                    parsing::ParseResult::Complete(r) => break r,
                    parsing::ParseResult::Partial(b) => {
                        buffer = io::Cursor::new(b);
                        continue;
                    }
                }
            },
            Err(e) => {
                if e.kind() != io::ErrorKind::WouldBlock && e.kind() != io::ErrorKind::TimedOut {
                    return Err(e.into());
                }

                if timeout.is_some() &&
                    elapsed_milliseconds(&start_time) >
                        duration_to_milliseconds(&timeout.unwrap())
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

    static HTTP_REQUEST: &'static [u8] = include_bytes!("../tests/big-http-request.txt");

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
        fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
            use std::thread;

            if let Some(timeout) = self.timeout {
                thread::sleep(timeout);
                Err(io::Error::new(io::ErrorKind::TimedOut, ""))
            } else {
                let read = match self.read_count {
                    0 => {
                        let half = self.content.len() / 2;
                        io::copy(&mut &self.content[..half], &mut buf)?
                    }
                    _ => {
                        let rest = self.bytes_read;
                        io::copy(&mut &self.content[rest..], &mut buf)?
                    }
                };

                self.bytes_read += read as usize;
                self.read_count += 1;

                Ok(read as _)
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
}
