extern crate simple_server;
extern crate futures;

use simple_server::Server;
use futures::future::ok;

#[test]
fn test_server_new() {
    Server::new(|_request, mut response| {
        Ok(Box::new(ok(response.body("Hello Rust!".as_bytes())?)))
    });
}

#[test]
fn test_error_fallback() {
    Server::new(|_request, mut response| {
        // set an invalid header
        response.header("Foo", "Bar\r\n");

        // this will then fail
        Ok(Box::new(ok(response.body("".as_bytes())?)))
    });
}
