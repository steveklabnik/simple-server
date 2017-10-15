extern crate simple_server;

use simple_server::{Server, Request, Response, StatusCode};
use simple_server::response::Builder as ResponseBuilder;
use simple_server::Method;

fn main() {
    let host = "127.0.0.1";
    let port = "7878";

    fn handler(request: Request<&[u8]>, mut response_builder: ResponseBuilder) -> Response<&[u8]> {
        println!("Request received. {} {}", request.method(), request.uri());

        match (request.method(), request.uri().path()) {
            (&Method::GET, "/hello") => {
                response_builder.body("<h1>Hi!</h1><p>Hello Rust!</p>".as_bytes()).unwrap()
            }
            (_, _) => {
                response_builder.status(StatusCode::NOT_FOUND);
                response_builder.body("<h1>404</h1><p>Not found!<p>".as_bytes()).unwrap()
            }
        }
    };

    let server = Server::new(handler);

    server.listen(host, port)
}
