use std::convert::Infallible;

pub use http::Request as HttpRequest;
pub use http_body_util::Full as HttpFull;
pub use http_body_util::combinators::BoxBody as HttpBoxBody;
pub use hyper::Response as HyperResponse;
pub use hyper::body::Body as HttpBody;
pub use hyper::body::Bytes as HyperBytes;
pub use hyper::http::response::Builder as HyperResponseBuilder;
pub use tokio_util::io::StreamReader as HttpStreamReader;

pub type HyperBytesResponse = HttpBoxBody<HyperBytes, Infallible>;
pub type HyperInfallibleResponse = HyperResponse<HyperBytesResponse>;
