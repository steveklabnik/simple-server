extern crate simple_server;

use simple_server::{Server, Request, Response};

fn main() {
    let host = "127.0.0.1";
    let port = "7878";

    fn handler(request: Request<&[u8]>, response: &mut Response<&[u8]>) {
        println!("Request received. {} {}", request.method(), request.uri());

        match (request.method(), request.uri().path()) {
            (&simple_server::method::GET, "/hello") => {
                *response.body_mut() = "<h1>Hi!</h1><p>Hello Rust!</p>".as_bytes();
            }
            (_, _) => {
                *response.status_mut() = simple_server::status::NOT_FOUND;
                *response.body_mut() = "<h1>404</h1><p>Not found!<p>".as_bytes();
            }
        }
    };

    let server = Server::new(handler);

    server.listen(host, port)
}
