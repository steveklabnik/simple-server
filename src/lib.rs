//! The `simple-server` crate is designed to give you the tools to
// to build an HTTP server, based around blocking I/O plus a threadpool.

extern crate http;
extern crate httparse;
extern crate scoped_threadpool;

pub use http::{response, status, Request, Response};
pub use http::status::StatusCode;
pub use http::method::Method;
use http::response::Builder as ResponseBuilder;

use scoped_threadpool::Pool;

use std::fs::File;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::path::Path;

mod error;

use error::Error;

/// Represents a server.
///
/// | Member    | Type                                       | Notes                                                               |
/// |-----------|--------------------------------------------|---------------------------------------------------------------------|
/// | `handler` | `fn(Request<&[u8]>, &mut Response<&[u8]>)` | This function uses Types that are re-exported from the `http` crate |
pub struct Server {
    handler: fn(Request<&[u8]>, ResponseBuilder) -> Result<Response<&[u8]>, Error>,
}


impl Server {
    /// Constructs a new server.
    pub fn new(
        handler: fn(Request<&[u8]>, ResponseBuilder) -> Result<Response<&[u8]>, Error>,
    ) -> Server {
        Server { handler }
    }

    fn handle_connection(&self, mut stream: TcpStream) -> Result<(), Error> {
        let mut buffer = [0; 512];

        stream.read(&mut buffer)?;

        let request = parse_request(&buffer)?;
        let mut response_builder = Response::builder();

        // first, we serve static files
        let fs_path = format!("public{}", request.uri());

        // ... you trying to do something bad?
        if fs_path.contains("./") || fs_path.contains("../") {
            // GET OUT
            response_builder.status(StatusCode::NOT_FOUND);

            let response = response_builder
                .body("<h1>404</h1><p>Not found!<p>".as_bytes())
                .unwrap();

            write_response(response, stream)?;
            return Ok(());
        }

        if Path::new(&fs_path).is_file() {
            let mut f = File::open(&fs_path)?;

            let mut source = Vec::new();

            f.read_to_end(&mut source)?;

            let response = response_builder.body(&*source)?;

            write_response(response, stream)?;
            return Ok(());
        }

        let response = (self.handler)(request, response_builder).unwrap_or_else(|_| {
            let mut response_builder = Response::builder();
            response_builder.status(StatusCode::INTERNAL_SERVER_ERROR);

            response_builder
                .body("<h1>500</h1><p>Internal Server Error!<p>".as_bytes())
                .unwrap()
        });

        Ok(write_response(response, stream)?)
    }

    /// Tells the server to listen on a specified host and port.
    pub fn listen(&self, host: &str, port: &str) {
        let mut pool = Pool::new(4);
        let listener =
            TcpListener::bind(format!("{}:{}", host, port)).expect("Error starting the server.");

        println!("Server started at http://{}:{}", host, port);

        for stream in listener.incoming() {
            let stream = stream.expect("Error handling TCP stream.");

            pool.scoped(|scope| {
                scope.execute(|| {
                    self.handle_connection(stream).expect(
                        "Error handling connection.",
                    );
                });
            });
        }
    }
}

fn write_response(response: Response<&[u8]>, mut stream: TcpStream) -> Result<(), Error> {
    let text =
        format!(
        "HTTP/1.1 {} {}\r\n\r\n",
        response.status().as_str(),
        response.status().canonical_reason().unwrap(),
    );
    stream.write(text.as_bytes())?;

    stream.write(response.body())?;
    Ok(stream.flush()?)
}

fn parse_request(raw_request: &[u8]) -> Result<Request<&[u8]>, Error> {
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);

    let header_length = req.parse(raw_request)?.unwrap() as usize;

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
