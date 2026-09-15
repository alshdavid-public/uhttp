use std::collections::HashMap;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::StatusCode;

const AUTHORIZATION: &str = "authorization";
const WWW_AUTHENTICATE: &str = "www-authenticate";
const BASIC_SCHEME: &str = "basic ";

/// Configuration for the [`basic_auth`] middleware.
///
/// Users are stored as a username to password mapping.
#[derive(Default, Clone)]
pub struct BasicAuthOptions {
  users: HashMap<String, String>,
  realm: String,
}

impl BasicAuthOptions {
  pub fn new() -> Self {
    Self {
      users: HashMap::new(),
      realm: "Restricted".to_string(),
    }
  }

  /// Add a user that is allowed to authenticate.
  pub fn user(
    mut self,
    username: impl Into<String>,
    password: impl Into<String>,
  ) -> Self {
    self.users.insert(username.into(), password.into());
    self
  }

  pub fn realm(
    mut self,
    realm: impl Into<String>,
  ) -> Self {
    self.realm = realm.into();
    self
  }
}

/// Middleware that enforces HTTP Basic authentication
///
/// Requests must carry a valid `Authorization: Basic base64(user:pass)`
/// header. Requests that are missing credentials or present invalid ones are
/// rejected with `401 Unauthorized` and a `WWW-Authenticate` challenge; the
/// inner handler is never invoked in that case.
pub fn basic_auth<T: 'static + Clone + Send + Sync>(
  options: BasicAuthOptions
) -> crate::router::RouterMiddlewareFunc<T> {
  let options = Arc::new(options);
  Box::new(move |req, res, ctx, next| {
    let options = Arc::clone(&options);
    Box::pin(async move {
      let challenge = format!("Basic realm=\"{}\", charset=\"UTF-8\"", options.realm);

      let header = req
        .headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(decode_credentials);

      let authorized = match header {
        Some((username, password)) => {
          matches!(options.users.get(&username), Some(expected) if *expected == password)
        }
        None => false,
      };

      if authorized {
        return next.run(req, res, ctx).await;
      }

      res
        .status(StatusCode::UNAUTHORIZED)
        .header(WWW_AUTHENTICATE, challenge)
        .body("Unauthorized")
    })
  })
}

fn decode_credentials(header: &str) -> Option<(String, String)> {
  let encoded = header
    .strip_prefix(BASIC_SCHEME)
    .or_else(|| header.strip_prefix("Basic "))?;

  let bytes = STANDARD.decode(encoded.trim()).ok()?;
  let decoded = String::from_utf8(bytes).ok()?;
  let (username, password) = decoded.split_once(':')?;

  Some((username.to_string(), password.to_string()))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn decodes_credentials() {
    // base64("admin:hunter2")
    let header = "Basic YWRtaW46aHVudGVyMg==";
    assert_eq!(
      decode_credentials(header),
      Some(("admin".to_string(), "hunter2".to_string()))
    );
  }

  #[test]
  fn allows_colons_in_password() {
    // base64("admin:pass:word")
    let header = "Basic YWRtaW46cGFzczp3b3Jk";
    assert_eq!(
      decode_credentials(header),
      Some(("admin".to_string(), "pass:word".to_string()))
    );
  }

  #[test]
  fn rejects_non_basic_scheme() {
    assert_eq!(decode_credentials("Bearer abc123"), None);
  }

  #[test]
  fn rejects_invalid_base64() {
    assert_eq!(decode_credentials("Basic not-base64!"), None);
  }

  #[test]
  fn rejects_payload_without_separator() {
    // base64("nocolon")
    assert_eq!(decode_credentials("Basic bm9jb2xvbg=="), None);
  }
}
