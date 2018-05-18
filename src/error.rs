use http;
use httparse;
use std;

/// Various errors that may happen while handling requests.
#[derive(Debug)]
pub enum Error {
    /// An error while doing I/O.
    Io(std::io::Error),
    /// An HTTP error.
    Http(http::Error),
    /// An error while parsing the HTTP request.
    HttpParse(httparse::Error),
    /// An error while parsing the URI of the request.
    InvalidUri(http::uri::InvalidUri),
    /// The request timed out.
    Timeout,
    #[doc(hidden)]
    RequestIncomplete,
    /// The request's size (headers + body) exceeded the application's limit.
    RequestTooLarge,
    /// The connection was closed while reading the request.
    ConnectionClosed,
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
