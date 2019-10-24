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
extern crate time;

pub use http::method::Method;
pub use http::response::Builder as ResponseBuilder;
pub use http::response::{Builder, Parts, Response};
pub use http::status::{InvalidStatusCode, StatusCode};
pub use http::Request;

use scoped_threadpool::Pool;

use std::env;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use std::borrow::Borrow;

mod error;
mod parsing;
mod request;

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
    static_directory: Option<PathBuf>,
}

impl fmt::Debug for Server {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Server {{ timeout: {:?}, static_directory: {:?} }}",
            self.timeout, self.static_directory
        )
    }
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
            static_directory: Some(PathBuf::from("public")),
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
            static_directory: Some(PathBuf::from("public")),
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
    /// # Panics
    ///
    /// There are several circumstances in which `listen` can currently panic:
    ///
    /// * If there's an error [constructing a TcpListener][constructing], generally if the port
    ///    or host is incorrect. See `TcpListener`'s docs for more.
    /// * If the connection fails, see [`incoming`'s docs] for more.
    ///
    /// Finally, if reading from the stream fails. Timeouts and connection closes
    /// are handled, other errors may result in a panic. This will only take down
    /// one of the threads in the threadpool, rather than the whole server.
    ///
    /// [constructing]: https://doc.rust-lang.org/std/net/struct.TcpListener.html#method.bind
    /// [`incoming`'s docs]: https://doc.rust-lang.org/std/net/struct.TcpListener.html#method.incoming
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

    /// Sets the proper directory for serving static files.
    ///
    /// By default, the server will serve static files inside a `public`
    /// directory. This method lets you set a path to whatever location
    /// you'd like.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// extern crate simple_server;
    ///
    /// use simple_server::Server;
    ///
    /// fn main() {
    ///     let mut server = Server::new(|request, mut response| {
    ///         Ok(response.body("Hello, world!".as_bytes().to_vec())?)
    ///     });
    ///
    ///     server.set_static_directory("/var/www/");
    ///
    ///     server.listen("127.0.0.1", "7979");
    /// }
    /// ```
    pub fn set_static_directory<P: Into<PathBuf>>(&mut self, path: P) {
        self.static_directory = Some(path.into());
    }

    /// Disables serving static files.
    ///
    /// By default, the server will serve static files inside a `public`
    /// directory, or the directory set by `set_static_directory`. This
    /// method lets you disable this.
    ///
    /// It can be re-enabled by a subsequent call to `set_static_directory`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// extern crate simple_server;
    ///
    /// use simple_server::Server;
    ///
    /// fn main() {
    ///     let mut server = Server::new(|request, mut response| {
    ///         Ok(response.body("Hello, world!".as_bytes().to_vec())?)
    ///     });
    ///
    ///     server.dont_serve_static_files();
    ///
    ///     server.listen("127.0.0.1", "7979");
    /// }
    /// ```
    pub fn dont_serve_static_files(&mut self) {
        self.static_directory = None;
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
            Err(Error::Io(ref io_error)) if io_error.kind() == std::io::ErrorKind::BrokenPipe => {
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
        if let Some(ref static_directory) = self.static_directory {
            let fs_path = request.uri().to_string();

            // the uri always includes a leading /, which means that join will over-write the static directory...
            let fs_path = PathBuf::from(&fs_path[1..]);

            // ... you trying to do something bad?
            let traversal_attempt = fs_path.components().any(|component| match component {
                std::path::Component::Normal(_) => false,
                _ => true,
            });

            if traversal_attempt {
                // GET OUT
                response_builder.status(StatusCode::NOT_FOUND);

                let response = response_builder
                    .body("<h1>404</h1><p>Not found!<p>".as_bytes())
                    .unwrap();

                write_response(response, stream)?;
                return Ok(());
            }

            let fs_path = static_directory.join(fs_path);

            if Path::new(&fs_path).is_file() {
                let mut f = File::open(&fs_path)?;

                let mut source = Vec::new();

                f.read_to_end(&mut source)?;

                let response = response_builder.body(source)?;

                write_response(response, stream)?;
                return Ok(());
            }
        }

        match (self.handler)(request, response_builder) {
            Ok(response) => Ok(write_response(response, stream)?),
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

fn write_response<T: Borrow<[u8]>, S: Write>(
    response: Response<T>,
    mut stream: S,
) -> Result<(), Error> {
    use fmt::Write;

    let (parts, body) = response.into_parts();
    let body: &[u8] = body.borrow();

    let mut text = format!(
        "HTTP/1.1 {} {}\r\n",
        parts.status.as_str(),
        parts
            .status
            .canonical_reason()
            .expect("Unsupported HTTP Status"),
    );

    if !parts.headers.contains_key(http::header::DATE) {
        let date = time::strftime("%a, %d %b %Y %H:%M:%S GMT", &time::now_utc()).unwrap();
        write!(text, "date: {}\r\n", date).unwrap();
    }
    if !parts.headers.contains_key(http::header::CONNECTION) {
        write!(text, "connection: close\r\n").unwrap();
    }
    if !parts.headers.contains_key(http::header::CONTENT_LENGTH) {
        write!(text, "content-length: {}\r\n", body.len()).unwrap();
    }
    for (k, v) in parts.headers.iter() {
        write!(text, "{}: {}\r\n", k.as_str(), v.to_str().unwrap()).unwrap();
    }

    write!(text, "\r\n").unwrap();

    stream.write(text.as_bytes())?;
    stream.write(body)?;
    Ok(stream.flush()?)
}

#[test]
fn test_write_response() {
    let mut builder = http::response::Builder::new();
    builder.status(http::StatusCode::OK);
    builder.header(http::header::DATE, "Thu, 01 Jan 1970 00:00:00 GMT");
    builder.header(http::header::CONTENT_TYPE, "text/plain".as_bytes());

    let mut output = vec![];
    let _ = write_response(builder.body("Hello rust".as_bytes()).unwrap(), &mut output).unwrap();
    let expected = b"HTTP/1.1 200 OK\r\n\
        connection: close\r\n\
        content-length: 10\r\n\
        date: Thu, 01 Jan 1970 00:00:00 GMT\r\n\
        content-type: text/plain\r\n\
        \r\n\
        Hello rust";
    assert_eq!(&expected[..], &output[..]);
}

#[test]
fn test_write_response_no_headers() {
    let mut builder = http::response::Builder::new();
    // Well, no headers besides the date ;) Otherwise, we wouldn't know
    // what `expected` should be.
    builder.header(http::header::DATE, "Thu, 01 Jan 1970 00:00:00 GMT");
    builder.status(http::StatusCode::OK);

    let mut output = vec![];
    let _ = write_response(builder.body("Hello rust".as_bytes()).unwrap(), &mut output).unwrap();
    let expected = b"HTTP/1.1 200 OK\r\n\
        connection: close\r\n\
        content-length: 10\r\n\
        date: Thu, 01 Jan 1970 00:00:00 GMT\r\n\
        \r\n\
        Hello rust";
    assert_eq!(&expected[..], &output[..]);
}
