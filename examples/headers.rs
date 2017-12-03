#[macro_use]
extern crate log;
extern crate env_logger;

extern crate http;
extern crate simple_server;

use simple_server::Server;
use http::header;

fn main() {
    env_logger::init().unwrap();

    let host = "127.0.0.1";
    let port = "7878";

    let server = Server::new(|request, mut response| {
        info!("Request received. {} {}", request.method(), request.uri());
        response.header(header::CONTENT_TYPE, "text/plain".as_bytes());
        Ok(response.body("Hello Rust!".as_bytes())?)
    });

    server.listen(host, port);
}
