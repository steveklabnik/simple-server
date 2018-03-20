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
pub use http::response::{Builder, Parts, Response};
pub use http::status::{InvalidStatusCode, StatusCode};
pub use http::method::Method;
pub use http::response::Builder as ResponseBuilder;

use scoped_threadpool::Pool;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use std::borrow::Cow;

mod error;
mod request;
mod parsing;

pub use error::Error;

pub type ResponseResult = Result<Response<Vec<u8>>, Error>;

pub type Handler =
    Box<Fn(Request<Vec<u8>>, ResponseBuilder) -> ResponseResult + 'static + Send + Sync>;

/// A web server.
///
/// This is the core type of this crate, and is used to create a new
/// server and listen for connections.
pub struct Server {
    handler: Handler,
    timeout: Option<Duration>,
    static_directory: PathBuf,
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
    ///         Ok(response.body("Hello, world!".as_bytes().to_vec())?)
    ///     });
    /// }
    /// ```
    pub fn new<H>(handler: H) -> Server
    where
        H: Fn(Request<Vec<u8>>, ResponseBuilder) -> ResponseResult + 'static + Send + Sync,
    {
        Server {
            handler: Box::new(handler),
            timeout: None,
            static_directory: PathBuf::from("public"),
        }
    }

    /// Constructs a new server with the given handler and the specified request
    /// timeout.
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
    /// use std::time::Duration;
    /// use simple_server::Server;
    ///
    /// fn main() {
    ///     let server = Server::with_timeout(Duration::from_secs(5), |request, mut response| {
    ///         Ok(response.body("Hello, world!".as_bytes().to_vec())?)
    ///     });
    /// }
    /// ```
    pub fn with_timeout<H>(timeout: Duration, handler: H) -> Server
    where
        H: Fn(Request<Vec<u8>>, ResponseBuilder) -> ResponseResult + 'static + Send + Sync,
    {
        Server {
            handler: Box::new(handler),
            timeout: Some(timeout),
            static_directory: PathBuf::from("public"),
        }
    }

    /// Tells the server to listen on a specified host and port.
    ///
    /// A threadpool is created, and used to handle connections.
    /// The pool size is four threads.
    ///
    /// This method blocks forever.
    ///
    /// The `listen` method will also serve static files. By default, that
    /// directory is "public" in the same directory as where it's run. If you'd like to change
    /// this default, please see the `set_static_directory` method.
    ///
    /// If someone tries a path directory traversal attack, this will return a
    /// `404`. Please note that [this is a best effort][best effort] at the
    /// moment.
    ///
    /// [best effort]: https://github.com/steveklabnik/simple-server/issues/54
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
    ///         Ok(response.body("Hello, world!".as_bytes().to_vec())?)
    ///     });
    ///
    ///     server.listen("127.0.0.1", "7979");
    /// }
    /// ```
    pub fn listen(&self, host: &str, port: &str) -> ! {
        let listener =
            TcpListener::bind(format!("{}:{}", host, port)).expect("Error starting the server.");

        info!("Server started at http://{}:{}", host, port);

        self.listen_on_socket(listener)
    }

    /// Tells the server to listen on a provided `TcpListener`.
    ///
    /// A threadpool is created, and used to handle connections.
    /// The pool size is four threads.
    ///
    /// This method blocks forever.
    ///
    /// This method will also serve static files out of a `public` directory
    /// in the same directory as where it's run. If someone tries a path
    /// directory traversal attack, this will return a `404`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// extern crate simple_server;
    ///
    /// use simple_server::Server;
    /// use std::net::TcpListener;
    ///
    /// fn main() {
    ///     let listener = TcpListener::bind(("127.0.0.1", 7979))
    ///         .expect("Error starting the server.");
    ///
    ///     let server = Server::new(|request, mut response| {
    ///         Ok(response.body("Hello, world!".as_bytes().to_vec())?)
    ///     });
    ///
    ///     server.listen_on_socket(listener);
    /// }
    /// ```
    pub fn listen_on_socket(&self, listener: TcpListener) -> ! {
        const READ_TIMEOUT_MS: u64 = 20;
        let num_threads = self.pool_size();
        let mut pool = Pool::new(num_threads);
        let mut incoming = listener.incoming();

        loop {
            // Incoming is an endless iterator, so it's okay to unwrap on it.
            let stream = incoming.next().unwrap();
            let stream = stream.expect("Error handling TCP stream.");

            stream
                .set_read_timeout(Some(Duration::from_millis(READ_TIMEOUT_MS)))
                .expect("FATAL: Couldn't set read timeout on socket");

            pool.scoped(|scope| {
                scope.execute(|| {
                    self.handle_connection(stream)
                        .expect("Error handling connection.");
                });
            });
        }
    }

    pub fn set_static_directory<P: Into<PathBuf>>(&mut self, path: P) {
        self.static_directory = path.into();
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
        let request = match request::read(&mut stream, self.timeout) {
            Err(Error::ConnectionClosed) | Err(Error::Timeout) | Err(Error::HttpParse(_)) => {
                return Ok(())
            }

            Err(Error::RequestTooLarge) => {
                let resp = Response::builder()
                    .status(StatusCode::PAYLOAD_TOO_LARGE)
                    .body("<h1>413</h1><p>Request too large!<p>".as_bytes())
                    .unwrap();
                write_response(resp, stream)?;
                return Ok(());
            }

            Err(e) => return Err(e),

            Ok(r) => r,
        };

        let mut response_builder = Response::builder();

        // first, we serve static files
        let fs_path = request.uri().to_string();

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

        // the uri always includes a leading /, which means that join will over-write the static directory...
        let fs_path = self.static_directory.join(&fs_path[1..]);

        if Path::new(&fs_path).is_file() {
            let mut f = File::open(&fs_path)?;

            let mut source = Vec::new();

            f.read_to_end(&mut source)?;

            let response = response_builder.body(source)?;

            write_response(response, stream)?;
            return Ok(());
        }

        match (self.handler)(request, response_builder) {
            Ok(mut response) => {
                let len = response.body().len().to_string();
                response.headers_mut().insert(
                    http::header::CONTENT_LENGTH,
                    http::header::HeaderValue::from_str(&len).unwrap(),
                );
                Ok(write_response(response, stream)?)
            }
            Err(_) => {
                let mut response_builder = Response::builder();
                response_builder.status(StatusCode::INTERNAL_SERVER_ERROR);

                let response = response_builder
                    .body("<h1>500</h1><p>Internal Server Error!<p>".as_bytes())
                    .unwrap();

                Ok(write_response(response, stream)?)
            }
        }
    }
}

fn write_response<'a, T: Into<Cow<'a, [u8]>>, S: Write>(
    response: Response<T>,
    mut stream: S,
) -> Result<(), Error> {
    let headers = response
        .headers()
        .iter()
        .fold(String::new(), |builder, (k, v)| {
            format!("{}{}: {}\r\n", builder, k.as_str(), v.to_str().unwrap())
        });

    let text = format!(
        "HTTP/1.1 {} {}\r\n{}\r\n",
        response.status().as_str(),
        response
            .status()
            .canonical_reason()
            .expect("Unsupported HTTP Status"),
        headers,
    );
    stream.write(text.as_bytes())?;

    let body: Cow<'a, [u8]> = {
        let (_, body) = response.into_parts();
        body.into()
    };

    stream.write(&*body)?;
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
