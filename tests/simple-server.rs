extern crate simple_server;

use simple_server::Server;

#[test]
fn test_server_new() {
    Server::new(|_request, mut response| Ok(response.body("Hello Rust!".as_bytes().to_vec())?));
}

#[test]
fn test_error_fallback() {
    Server::new(|_request, mut response| {
        // set an invalid header
        response.header("Foo", "Bar\r\n");

        // this will then fail
        Ok(response.body("".as_bytes().to_vec())?)
    });
}
