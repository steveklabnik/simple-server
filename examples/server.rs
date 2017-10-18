#[macro_use]
extern crate log;
extern crate env_logger;
extern crate futures;

extern crate simple_server;

use simple_server::Server;
use futures::future::ok;

fn main() {
    env_logger::init().unwrap();

    let host = "127.0.0.1";
    let port = "7878";

    let server = Server::new(|request, mut response| {
        info!("Request received. {} {}", request.method(), request.uri());
        Ok(Box::new(ok(response.body("Hello Rust!".as_bytes())?)))
    });

    server.listen(host, port);
}
