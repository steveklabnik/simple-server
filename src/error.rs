use std;
use http;
use httparse;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Http(http::Error),
    HttpParse(httparse::Error),
    InvalidUri(http::uri::InvalidUri),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<http::Error> for Error {
    fn from(err: http::Error) -> Error {
        Error::Http(err)
    }
}

impl From<httparse::Error> for Error {
    fn from(err: httparse::Error) -> Error {
        Error::HttpParse(err)
    }
}

impl From<http::uri::InvalidUri> for Error {
    fn from(err: http::uri::InvalidUri) -> Error {
        Error::InvalidUri(err)
    }
}
