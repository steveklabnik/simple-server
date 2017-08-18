extern crate simple_server;
extern crate http;

use simple_server::{Server, Request, Response};
use http::response::Builder as ResponseBuilder;

#[test]
fn test_new() {
    fn handler(_request: Request<&[u8]>, mut response_builder: ResponseBuilder) -> Response<&[u8]> {
        response_builder.body("Hello Rust!".as_bytes()).unwrap()
    };

    let _server = Server::new(handler);
}
