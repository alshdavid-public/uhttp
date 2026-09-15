use std::collections::HashMap;
use std::hash::DefaultHasher;
use std::hash::Hasher;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncSeekExt;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;

use crate::HandleFunc;
use crate::Request;
use crate::ResponseBody;
use crate::StatusCode;

/// Size of the buffer bodies are streamed through on their way to the client
const BODY_BUFFER_SIZE: usize = 64 * 1024;

#[derive(Debug, Default)]
pub enum ETagStrategy {
  /// Non cryptographic hash of the file, slower
  Hash,
  /// Faster
  #[default]
  LastModified,
  /// No etag calc
  Disabled,
}

#[derive(Debug)]
pub struct FileServerOptions {
  /// The root directory to get files from
  pub dir: PathBuf,
  /// Send back compressed responses
  pub compress: bool,
  /// How to supply etag
  pub etag: ETagStrategy,
  /// Add Custom Headers
  pub custom_headers: HashMap<String, String>,
  /// Relative path to fallback URL. Defaults to "404.html"
  pub fallback_route: Option<String>,
  /// Defaults to [`crate::StatusCode::NOT_FOUND] (404)
  pub fallback_status: Option<StatusCode>,
}

/// Serve files from the filesystem
pub fn create(options: FileServerOptions) -> HandleFunc {
  let options = Arc::new(options);
  let fallback_path = Arc::new(match options.fallback_route.as_ref() {
    Some(path) => options.dir.join(path),
    None => options.dir.join("404.html"),
  });
  let fallback_status = match options.fallback_status.as_ref() {
    Some(status) => *status,
    None => StatusCode::NOT_FOUND,
  };

  Box::new(move |req, mut res| {
    let options = Arc::clone(&options);
    let fallback_path = Arc::clone(&fallback_path);

    Box::pin(async move {
      let url_path = determine_file(req.uri.path());
      let mut extension = try_extension(&url_path)?;
      let mut status = StatusCode::OK;

      for (key, value) in &options.custom_headers {
        res = res.header(key.as_str(), value.as_str());
      }

      let mut file = match tokio::fs::File::open(&options.dir.join(&url_path)).await {
        Ok(file) => file,
        Err(_) => match tokio::fs::File::open(&*fallback_path).await {
          Ok(file) => {
            status = fallback_status;
            extension = try_extension(&fallback_path)?;
            file
          }
          Err(_) => return res.status(StatusCode::NOT_FOUND).body(""),
        },
      };

      let mime_type = mime_guess::from_ext(extension)
        .first_or_octet_stream()
        .to_string();

      res = res.header("Content-Type", mime_type.as_str());

      let encoding = negotiate_encoding(&req, options.compress);

      if let Some(content_encoding) = encoding.header_value() {
        res = res.header("Content-Encoding", content_encoding);
      }

      if let Some(etag) = etag_file(&mut file, &options.etag, encoding.etag_suffix()).await? {
        if !has_modified(&req, &etag) {
          return res.status(StatusCode::NOT_MODIFIED).body("");
        }
        res = res.header("ETag", etag.as_str());
      }

      let (body, mut writer) = ResponseBody::chunked(BODY_BUFFER_SIZE);

      // The body is written after the response head has been handed back, so the
      // copy has to outlive this future. A client hanging up mid transfer shows up
      // as a write error here and there is no longer anywhere to report it to.
      tokio::task::spawn(async move {
        let _ = match encoding {
          Encoding::Zstd => zstd_stream(&mut file, &mut writer).await,
          Encoding::Brotli => brotli_stream(&mut file, &mut writer).await,
          Encoding::Gzip => gzip_stream(&mut file, &mut writer).await,
          Encoding::Identity => tokio::io::copy(&mut file, &mut writer).await,
        };
        let _ = writer.shutdown().await;
      });

      res.status(status).body(body)
    })
  })
}

#[derive(Clone, Copy, Debug)]
enum Encoding {
  Zstd,
  Brotli,
  Gzip,
  Identity,
}

impl Encoding {
  /// Value to send back in the `Content-Encoding` header, if any
  fn header_value(&self) -> Option<&'static str> {
    match self {
      Self::Zstd => Some("zstd"),
      Self::Brotli => Some("br"),
      Self::Gzip => Some("gzip"),
      Self::Identity => None,
    }
  }

  /// Mixed into the etag so encodings of the same file don't share a cache entry
  fn etag_suffix(&self) -> &'static str {
    self.header_value().unwrap_or("")
  }
}

fn negotiate_encoding(
  req: &Request,
  compress: bool,
) -> Encoding {
  if !compress {
    return Encoding::Identity;
  }

  let Some(accept_encoding) = req.headers.get("Accept-Encoding") else {
    return Encoding::Identity;
  };

  let Ok(accept_encoding) = accept_encoding.to_str() else {
    return Encoding::Identity;
  };

  if accept_encoding.contains("zstd") {
    Encoding::Zstd
  } else if accept_encoding.contains("br") {
    Encoding::Brotli
  } else if accept_encoding.contains("gz") {
    Encoding::Gzip
  } else {
    Encoding::Identity
  }
}

fn has_modified(
  req: &Request,
  etag: &str,
) -> bool {
  if let Some(if_none_match) = req.headers.get("If-None-Match")
    && if_none_match == etag
  {
    return false;
  }
  true
}

fn determine_file(input: &str) -> PathBuf {
  if input == "/" {
    PathBuf::from("/index.html".trim_start_matches("/"))
  } else if PathBuf::from(input).extension().is_some() {
    PathBuf::from(input.trim_start_matches("/"))
  } else {
    PathBuf::from(format!("{}.html", input.trim_start_matches("/")))
  }
}

async fn etag_file(
  file: &mut tokio::fs::File,
  strategy: &ETagStrategy,
  encoding: &str,
) -> Result<Option<String>, std::io::Error> {
  match strategy {
    ETagStrategy::Hash => {
      let file_handle_copy = file.try_clone().await?;
      let mut reader = BufReader::new(file_handle_copy);
      let mut hasher = DefaultHasher::new();
      let mut buffer = [0u8; 64 * 1024];

      loop {
        let n = reader.read(&mut buffer).await?;
        if n == 0 {
          break;
        }
        hasher.write(&buffer[..n]);
      }

      file.seek(std::io::SeekFrom::Start(0)).await?;

      Ok(Some(format!("{:016x}{}", hasher.finish(), encoding)))
    }
    ETagStrategy::LastModified => {
      let meta = file.metadata().await?;
      let etag = format!(
        "{:x}{:x}{}",
        meta
          .modified()?
          .duration_since(std::time::UNIX_EPOCH)
          .unwrap()
          .as_nanos(),
        meta.len(),
        encoding,
      );
      Ok(Some(etag))
    }
    ETagStrategy::Disabled => Ok(None),
  }
}

async fn gzip_stream<R, W>(
  input: R,
  output: &mut W,
) -> Result<u64, std::io::Error>
where
  R: AsyncRead + Unpin,
  W: AsyncWrite + Unpin,
{
  use async_compression::tokio::bufread::GzipEncoder;
  let mut encoder = GzipEncoder::new(BufReader::new(input));
  tokio::io::copy(&mut encoder, output).await
}

async fn brotli_stream<R, W>(
  input: R,
  output: &mut W,
) -> Result<u64, std::io::Error>
where
  R: AsyncRead + Unpin,
  W: AsyncWrite + Unpin,
{
  use async_compression::tokio::bufread::BrotliEncoder;
  let mut encoder = BrotliEncoder::new(BufReader::new(input));
  tokio::io::copy(&mut encoder, output).await
}

async fn zstd_stream<R, W>(
  input: R,
  output: &mut W,
) -> Result<u64, std::io::Error>
where
  R: AsyncRead + Unpin,
  W: AsyncWrite + Unpin,
{
  use async_compression::tokio::bufread::ZstdEncoder;
  let mut encoder = ZstdEncoder::new(BufReader::new(input));
  tokio::io::copy(&mut encoder, output).await
}

fn try_extension(input: &Path) -> crate::Result<&str> {
  let Some(ext) = input.extension() else {
    return Ok(Default::default());
  };

  let Some(ext) = ext.to_str() else {
    return Ok(Default::default());
  };

  Ok(ext)
}
