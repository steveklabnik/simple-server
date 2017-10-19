//! A simple webserver.
//!
//! The `simple-server` crate is designed to give you the tools to to build
//! an HTTP server, based around the http crate, blocking I/O, and a
//! threadpool.
//!
//! We call it 'simple' want to keep the code small, and easy to
//! understand. This is why we're only using blocking I/O. Depending on
//! your needs, you may or may not want to choose another server.
//! However, just the simple stuff is often enough for many projects.
//!
//! # Examples
//!
//! At its core, `simple-server` contains a `Server`. The `Server` is
//! passed a handler upon creation, and the `listen` method is used
//! to start handling connections.
//!
//! The other types are from the `http` crate, and give you the ability
//! to work with various aspects of HTTP. The `Request`, `Response`, and
//! `ResponseBuilder` types are used by the handler you give to `Server`,
//! for example.
//!
//! To see examples of this crate in use, please consult the `examples`
//! directory.

#[macro_use]
extern crate log;

extern crate http;
extern crate httparse;
extern crate num_cpus;
extern crate scoped_threadpool;

pub use http::Request;
pub use http::response::{Builder, Response, Parts};
pub use http::status::{InvalidStatusCode, StatusCode};
pub use http::method::Method;
pub use http::response::Builder as ResponseBuilder;

use scoped_threadpool::Pool;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::path::Path;

mod error;

pub use error::Error;

/// A web server.
///
/// This is the core type of this crate, and is used to create a new
/// server and listen for connections.
pub struct Server {
    handler: fn(Request<&[u8]>, ResponseBuilder) -> Result<Response<&[u8]>, Error>,
}


impl Server {
    /// Constructs a new server with the given handler.
    ///
    /// The handler function is called on all requests.
    ///
    /// # Errors
    ///
    /// The handler function returns a `Result` so that you may use `?` to
    /// handle errors. If a handler returns an `Err`, a 500 will be shown.
    ///
    /// If you'd like behavior other than that, return an `Ok(Response)` with
    /// the proper error code. In other words, this behavior is to gracefully
    /// handle errors you don't care about, not for properly handling
    /// non-`HTTP 200` responses.
    ///
    /// # Examples
    ///
    /// ```
    /// extern crate simple_server;
    ///
    /// use simple_server::Server;
    ///
    /// fn main() {
    ///     let server = Server::new(|request, mut response| {
    ///         Ok(response.body("Hello, world!".as_bytes())?)
    ///     });
    /// }
    /// ```
    pub fn new(
        handler: fn(Request<&[u8]>, ResponseBuilder) -> Result<Response<&[u8]>, Error>,
    ) -> Server {
        Server { handler }
    }

    /// Tells the server to listen on a specified host and port.
    ///
    /// A threadpool is created, and used to handle connections.
    /// The pool size is four threads.
    ///
    /// This method blocks forever.
    ///
    /// The `listen` method will also serve static files out of a `public`
    /// directory in the same directory as where it's run. If someone tries
    /// a path directory traversal attack, this will return a `404`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// extern crate simple_server;
    ///
    /// use simple_server::Server;
    ///
    /// fn main() {
    ///     let server = Server::new(|request, mut response| {
    ///         Ok(response.body("Hello, world!".as_bytes())?)
    ///     });
    ///
    ///     server.listen("127.0.0.1", "7979");
    /// }
    /// ```
    pub fn listen(&self, host: &str, port: &str) {
        let num_threads = self.pool_size();
        let mut pool = Pool::new(num_threads);
        let listener =
            TcpListener::bind(format!("{}:{}", host, port)).expect("Error starting the server.");

        info!("Server started at http://{}:{}", host, port);

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

    // Try and fetch the environment variable SIMPLESERVER_THREADS and parse it as a u32.
    // If this fails we fall back to using the num_cpus crate.
    fn pool_size(&self) -> u32 {
        const NUM_THREADS: &str = "SIMPLESERVER_THREADS";
        let logical_cores = num_cpus::get() as u32;

        match env::var(NUM_THREADS) {
            Ok(v) => v.parse::<u32>().unwrap_or(logical_cores),
            Err(_) => logical_cores,
        }
    }

    fn handle_connection(&self, mut stream: TcpStream) -> Result<(), Error> {
        let mut buffer = [0; 512];

        if stream.read(&mut buffer)? == 0 {
            // Connection closed
            return Ok(());
        }

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
}

fn write_response<T: Write>(response: Response<&[u8]>, mut stream: T) -> Result<(), Error> {
    let headers = response.headers().iter().fold(
        String::new(),
        |builder, (k, v)| {
            format!("{}{}: {}\r\n", builder, k.as_str(), v.to_str().unwrap())
        },
    );

    let text =
        format!(
        "HTTP/1.1 {} {}\r\n{}\r\n",
        response.status().as_str(),
        response
            .status()
            .canonical_reason()
            .expect("Unsupported HTTP Status"),
        headers,
    );
    stream.write(text.as_bytes())?;

    stream.write(response.body())?;
    Ok(stream.flush()?)
}

#[test]
fn test_write_response() {
    let mut builder = http::response::Builder::new();
    builder.status(http::StatusCode::OK);
    builder.header(http::header::CONTENT_TYPE, "text/plain".as_bytes());

    let mut output = vec![];
    let _ = write_response(builder.body("Hello rust".as_bytes()).unwrap(), &mut output).unwrap();
    let expected = b"HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\n\r\nHello rust";
    assert_eq!(&expected[..], &output[..]);
}

#[test]
fn test_write_response_no_headers() {
    let mut builder = http::response::Builder::new();
    builder.status(http::StatusCode::OK);

    let mut output = vec![];
    let _ = write_response(builder.body("Hello rust".as_bytes()).unwrap(), &mut output).unwrap();
    let expected = b"HTTP/1.1 200 OK\r\n\r\nHello rust";
    assert_eq!(&expected[..], &output[..]);
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
