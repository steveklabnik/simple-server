//! The `simple-server` crate is designed to give you the tools to
// to build an HTTP server, based around blocking I/O plus a threadpool.

extern crate http;
extern crate httparse;
extern crate scoped_threadpool;

pub use http::{status, method, Request, Response};

use http::{request, HeaderMap};
use http::header::HeaderValue;

use scoped_threadpool::Pool;

use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

/// Represents a server. 
///
/// | Member    | Type                                       | Notes                                                               |
/// |-----------|--------------------------------------------|---------------------------------------------------------------------|
/// | `handler` | `fn(Request<&[u8]>, &mut Response<&[u8]>)` | This function uses Types that are re-exported from the `http` crate |
pub struct Server {
    handler: fn(Request<&[u8]>, &mut Response<&[u8]>),
}


impl Server {
    /// Constructs a new server.
    pub fn new(handler: fn(Request<&[u8]>, &mut Response<&[u8]>)) -> Server {
        Server { handler }
    }

    fn handle_connection(&self, mut stream: TcpStream) {
        let mut buffer = [0; 512];

        stream.read(&mut buffer).unwrap();

        let request = parse_request(&buffer);
        let mut response = Response::default();
        (self.handler)(request, &mut response);
        write_response(response, stream);
    }

    /// Tells the server to listen on a specified host and port.
    pub fn listen(&self, host: &str, port: &str) {
        let mut pool = Pool::new(4);
        let listener = TcpListener::bind(format!("{}:{}", host, port)).unwrap();

        println!("Server started at http://{}:{}", host, port);

        for stream in listener.incoming() {
            let stream = stream.unwrap();

            pool.scoped(|scope| {
                scope.execute(|| { self.handle_connection(stream); });
            });
        }
    }
}

fn write_response(response: Response<&[u8]>, mut stream: TcpStream) {
    let text = format!(
        "HTTP/1.1 {} {}\r\n\r\n",
        response.status().as_str(),
        response.status().canonical_reason().unwrap()
    );
    stream.write(text.as_bytes()).unwrap();

    stream.write(response.body()).unwrap();
    stream.flush().unwrap();
}

fn parse_request(raw_request: &[u8]) -> Request<&[u8]> {
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);

    let header_length = req.parse(raw_request).unwrap().unwrap() as usize;

    let mut head = request::Head::default();
    let mut header_map = HeaderMap::new();

    for header in req.headers {
        header_map.insert(
            header.name,
            HeaderValue::try_from_bytes(header.value).unwrap(),
        );
    }

    head.headers = header_map;

    let body = &raw_request[header_length..];

    let mut request = Request::from_parts(head, body);
    let path = req.path.unwrap();
    *request.uri_mut() = path.parse().unwrap();

    request
}
