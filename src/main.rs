extern crate http;
extern crate scoped_threadpool;

use http::{request, Request, response, Response};

use scoped_threadpool::Pool;

use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

struct Server {
    handler: fn(Request<&[u8]>) -> Response<&[u8]>,
}

fn main() {
    let mut pool = Pool::new(4);
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    let server = Server {
        handler: handler,
    };

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.scoped(|scope| {
            scope.execute(|| {
                server.handle_connection(stream);
            });
        });
    }

}

impl Server {
    fn handle_connection(&self, mut stream: TcpStream) {
        let mut buffer = [0; 512];

        stream.read(&mut buffer).unwrap();

        println!("Request: {}", String::from_utf8_lossy(&buffer[..]));
        let request = parse_request(&buffer);
        let response = (self.handler)(request);
        write_response(response, stream);
    }
}

fn write_response(response: Response<&[u8]>, mut stream: TcpStream) {
    let text = format!("HTTP/1.1 {} {}\r\n\r\n", response.status().as_str(), response.status().canonical_reason().unwrap());
    stream.write(text.as_bytes()).unwrap();

    stream.write(response.body()).unwrap();
    stream.flush().unwrap(); 
}

fn parse_request(raw_request: &[u8]) -> Request<&[u8]> {
    let head = request::Head::default();
    let request = Request::from_parts(head, "hello world".as_bytes());

    request
}

fn handler(request: Request<&[u8]>) -> Response<&[u8]> {
    let head = response::Head::default();
    let response = Response::from_parts(head, "<h1>hello world</h1>".as_bytes());

    response
}