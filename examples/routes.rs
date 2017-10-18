#[macro_use]
extern crate log;
extern crate env_logger;
extern crate futures;

extern crate simple_server;

use futures::future::ok;

use simple_server::{Server, Method, StatusCode};

fn main() {
    let host = "127.0.0.1";
    let port = "7878";

    let server = Server::new(|request, mut response| {
        info!("Request received. {} {}", request.method(), request.uri());

        match (request.method(), request.uri().path()) {
            (&Method::GET, "/hello") => {
                Ok(Box::new(ok(response.body("<h1>Hi!</h1><p>Hello Rust!</p>".as_bytes())?)))
            }
            (_, _) => {
                response.status(StatusCode::NOT_FOUND);
                Ok(Box::new(ok(response.body("<h1>404</h1><p>Not found!<p>".as_bytes())?)))
            }
        }
    });

    server.listen(host, port);
}
