extern crate env_logger;
#[macro_use]
extern crate log;

extern crate simple_server;

use simple_server::{Method, Server, StatusCode};

fn main() {
    let host = "127.0.0.1";
    let port = "7878";

    let server = Server::new(|request, mut response| {
        info!("Request received. {} {}", request.method(), request.uri());

        match request.method() {
            &Method::GET => {
                let body = format!("The path you requested was '{}'", request.uri().path());
                Ok(response.body(body.into_bytes())?)
            }
            &Method::POST => {
                let data = String::from_utf8_lossy(request.body()).into_owned();
                let body = format!("The data you posted was '{}'", data);
                Ok(response.body(body.into_bytes())?)
            }
            _ => {
                response.status(StatusCode::NOT_FOUND);
                Ok(response.body(b"<h1>404</h1><p>Not found!<p>".to_vec())?)
            }
        }
    });

    server.listen(host, port);
}
