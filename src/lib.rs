extern crate http;
extern crate scoped_threadpool;

pub use http::{request, Request, response, Response};

use scoped_threadpool::Pool;

use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

pub struct Server {
    handler: fn(Request<&[u8]>, &mut Response<&[u8]>),
}


impl Server {
    pub fn new(handler: fn(Request<&[u8]>, &mut Response<&[u8]>)) -> Server {
        Server { handler }
    }

    fn handle_connection(&self, mut stream: TcpStream) {
        let mut buffer = [0; 512];

        stream.read(&mut buffer).unwrap();

        println!("Request: {}", String::from_utf8_lossy(&buffer[..]));
        let request = parse_request(&buffer);
        let mut response = Response::default();
        (self.handler)(request, &mut response);
        write_response(response, stream);
    }

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
    let head = request::Head::default();
    let request = Request::from_parts(head, "hello world".as_bytes());

    request
}
