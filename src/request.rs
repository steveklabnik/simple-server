use std::io::{self, Read};
use std::time::{Duration, Instant};
use error::Error;
use super::Request;

use httparse;

fn elapsed_milliseconds(from: &Instant) -> u64 {
    let elapsed = Instant::now() - *from;
    (elapsed.as_secs() * 1000) + (elapsed.subsec_nanos() as u64 / 1_000_000)
}

fn duration_to_milliseconds(from: &Duration) -> u64 {
    (from.as_secs() * 1000) + (from.subsec_nanos() as u64 / 1_000_000)
}

pub(super) fn read<'request, S: Read>(
    buf: &'request mut [u8],
    stream: &mut S,
    timeout: Option<Duration>,
) -> Result<Request<&'request [u8]>, Error> {
    let start_time = Instant::now();
    let mut total_read = 0;

    loop {
        if total_read == buf.len() {
            return Err(Error::RequestTooLarge);
        }

        let read = match stream.read(&mut buf[total_read..]) {
            Ok(num) if num == 0 => return Err(Error::ConnectionClosed),
            Ok(num) => num,
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
        };

        total_read += read;
        if is_valid_request(&buf[..total_read])? {
            break;
        }
    }

    Ok(parse_request(&buf[..total_read])?)
}

fn is_valid_request(data: &[u8]) -> Result<bool, Error> {
    use httparse::Status;

    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);

    match req.parse(data)? {
        Status::Complete(_) => Ok(true),
        Status::Partial => Ok(false),
    }
}

fn parse_request(raw_request: &[u8]) -> Result<Request<&[u8]>, Error> {
    use httparse::Status;

    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);

    let header_length = match req.parse(raw_request)? {
        Status::Complete(n) => n as usize,
        Status::Partial => return Err(Error::RequestIncomplete),
    };

    let body = &raw_request[header_length..];
    let mut http_req = Request::builder();

    for header in req.headers {
        http_req.header(header.name, header.value);
    }

    let mut request = http_req.body(body)?;
    let path = req.path.unwrap();
    *request.uri_mut() = path.parse()?;

    Ok(request)
}

#[cfg(test)]
mod server_should {

    use super::*;

    static HTTP_REQUEST: &'static [u8] = b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";

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

                Ok(read as usize)
            }
        }
    }

    #[test]
    fn read_request_stream_in_multiple_chunks() {
        let mut buf = [0_u8; 512];
        let mut s = ChunkStream::new(HTTP_REQUEST);

        assert!(read(&mut buf, &mut s, None).is_ok());
    }

    #[test]
    fn honour_request_timeout() {
        let timeout = Duration::from_millis(50);
        let mut buf = [0_u8; 512];
        let mut s = ChunkStream::with_timeout(HTTP_REQUEST, timeout);

        let result = read(&mut buf, &mut s, Some(timeout));

        match result {
            Err(Error::Timeout) => {}
            Err(e) => panic!("Expected timeout but got {:?}", e),
            Ok(_) => panic!("Expected timeout error but got Ok(_)"),
        }
    }
}
