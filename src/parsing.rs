use httparse;

struct RequestProtocolIndices {
    path: (usize, usize),
}

struct HeaderIndices {
    name: (usize, usize),
    value: (usize, usize),
}

pub struct Request {
    proto: RequestProtocolIndices,
    headers: Vec<HeaderIndices>,
    body: (usize, usize),
    buffer: Vec<u8>,
}

pub struct Header<'a> {
    pub name: &'a str,
    pub value: &'a [u8],
}

impl Request {
    pub fn split_body(&mut self) -> Vec<u8> {
        let body = self.buffer.drain(self.body.0..).collect();
        let (start, _) = self.body;
        self.body = (start, start);
        body
    }

    pub fn path(&self) -> &str {
        ::std::str::from_utf8(&self.buffer[self.proto.path.0..self.proto.path.1])
            .unwrap()
    }

    pub fn headers<'a>(&'a self) -> HeaderIter<'a> {
        HeaderIter(&self.buffer, self.headers.iter())
    }
}

pub struct HeaderIter<'a>(&'a [u8], ::std::slice::Iter<'a, HeaderIndices>);

impl<'a> Iterator for HeaderIter<'a> {
    type Item = Header<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        use std::str;
        self.1.next()
            .map(|&HeaderIndices{ref name, ref value}| {
                Header {
                    name: str::from_utf8(&self.0[name.0..name.1]).unwrap(),
                    value: &self.0[value.0..value.1],
                }
            })
    }
}

pub enum ParseResult {
    Complete(Request),
    Partial(Vec<u8>),
}

fn slice_indices(buffer: &[u8], value: &[u8]) -> (usize, usize) {
    let start = value.as_ptr() as usize - buffer.as_ptr() as usize;
    (start, start + value.len())
}

pub fn try_parse_request(buffer: Vec<u8>) -> Result<ParseResult, httparse::Error> {
    let result = {
        let mut header_buffer = [httparse::EMPTY_HEADER; 32];
        let mut request = httparse::Request::new(&mut header_buffer);
        let request = match request.parse(&*buffer)? {
            httparse::Status::Partial => None,
            httparse::Status::Complete(n) => Some((request, n)),
        };

        request.map(|(r, n)| {
            let proto = RequestProtocolIndices {
                path: slice_indices(&*buffer, r.path.unwrap().as_bytes()),
            };

            (r, proto, n)
        })
        .map(|(r, proto, n)| {
            let headers = r.headers.iter()
                .map(|&httparse::Header { ref name, ref value }| {
                    HeaderIndices {
                        name: slice_indices(&*buffer, name.as_bytes()),
                        value: slice_indices(&*buffer, value),
                    }
                })
                .collect::<Vec<_>>();
            (proto, headers, n)
        })
    };

    if let Some((proto, headers, n)) = result {
        return Ok(ParseResult::Complete(Request {
            proto: proto,
            headers: headers,
            body: slice_indices(&*buffer, &buffer[n..]),
            buffer: buffer
        }));
    }

    return Ok(ParseResult::Partial(buffer));
}

