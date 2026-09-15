use std::borrow::Cow;
use std::convert::Infallible;

use futures::TryStreamExt;
use tokio::io::DuplexStream;

pub enum ResponseBody {
  Bytes(Vec<u8>),
  BoxedBody(super::types::HyperBytesResponse),
}

impl From<Vec<u8>> for ResponseBody {
  fn from(value: Vec<u8>) -> Self {
    Self::Bytes(value)
  }
}

impl From<&[u8]> for ResponseBody {
  fn from(value: &[u8]) -> Self {
    Self::Bytes(value.to_vec())
  }
}

impl<const N: usize> From<&[u8; N]> for ResponseBody {
  fn from(value: &[u8; N]) -> Self {
    Self::Bytes(value.to_vec())
  }
}

impl<'a> From<Cow<'a, [u8]>> for ResponseBody {
  fn from(value: Cow<'a, [u8]>) -> Self {
    Self::Bytes(value.to_vec())
  }
}

impl From<&str> for ResponseBody {
  fn from(value: &str) -> Self {
    Self::Bytes(value.as_bytes().to_vec())
  }
}

impl From<String> for ResponseBody {
  fn from(value: String) -> Self {
    Self::Bytes(value.as_bytes().to_vec())
  }
}

impl From<super::types::HyperBytesResponse> for ResponseBody {
  fn from(value: super::types::HyperBytesResponse) -> Self {
    Self::BoxedBody(value)
  }
}

impl ResponseBody {
  pub fn chunked(max_buffer_size: usize) -> (Self, DuplexStream) {
    let (writer, reader) = tokio::io::duplex(max_buffer_size);

    let reader_stream = tokio_util::io::ReaderStream::new(reader)
      .map_ok(hyper::body::Frame::data)
      .map_err(|_item| panic!());

    let stream_body = http_body_util::StreamBody::new(reader_stream);
    let boxed_body =
      super::types::HttpBoxBody::<super::types::HyperBytes, Infallible>::new(stream_body);

    (Self::BoxedBody(boxed_body), writer)
  }
}
