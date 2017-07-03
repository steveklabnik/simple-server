extern crate simple_server;

use simple_server::{Server, Request, Response};

fn main() {
    let host = "127.0.0.1";
    let port = "7878";

    fn handler(request: Request<&[u8]>, response: &mut Response<&[u8]>) {
        println!("Request received. {} {}", request.method(), request.uri());
        *response.body_mut() = "Hello Rust!".as_bytes();
    };

    let server = Server::new(handler);

    server.listen(host, port)
}
