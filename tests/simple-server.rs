extern crate simple_server;

use simple_server::{Server, Request, Response};

#[test]
fn test_new() {
    fn handler(request: Request<&[u8]>, response: &mut Response<&[u8]>) {
        *response.body_mut() = "Hello Rust!".as_bytes();
    };

    let server = Server::new(handler);
}
