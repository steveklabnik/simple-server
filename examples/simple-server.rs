extern crate simple_server;

use simple_server::{Server, Request, Response};
use simple_server::response::Builder as ResponseBuilder;

fn main() {
    let host = "127.0.0.1";
    let port = "7878";

    fn handler(request: Request<&[u8]>, mut response_builder: ResponseBuilder) -> Response<&[u8]> {
        println!("Request received. {} {}", request.method(), request.uri());
        response_builder.body("Hello Rust!".as_bytes()).unwrap()
    };

    let server = Server::new(handler);

    server.listen(host, port)
}
