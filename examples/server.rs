#[macro_use]
extern crate log;
extern crate env_logger;

extern crate simple_server;

use simple_server::Server;

fn main() {
    env_logger::init().unwrap();

    let host = "127.0.0.1";
    let port = "7878";

    let server = Server::new(|request, mut response| {
        let msg = format!("The path you requested was: '{}'", request.uri().path());
        Ok(response.body(msg.into_bytes().into())?)
    });

    server.listen(host, port);
}
