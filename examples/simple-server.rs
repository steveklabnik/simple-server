extern crate simple_server;

use simple_server::Server;

fn main() {
    let host = "127.0.0.1";
    let port = "7878";

    let server = Server::new(|request, mut response| {
        println!("Request received. {} {}", request.method(), request.uri());
        Ok(response.body("Hello Rust!".as_bytes())?)
    });

    server.listen(host, port);
}
