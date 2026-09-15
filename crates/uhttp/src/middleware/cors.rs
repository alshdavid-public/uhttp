use http::Method;

use crate::router::Next;

/// Permissive CORS middleware.
///
/// Adds `Access-Control-Allow-*: *` headers to every response and answers
/// preflight (`OPTIONS`) requests with `204 No Content` without running the
/// inner handler.
pub async fn cors<T>(
  req: crate::Request,
  res: crate::Response,
  ctx: T,
  next: Next<T>,
) -> crate::HandlerResult {
  let res = res
    .header("access-control-allow-origin", "*")
    .header("access-control-allow-methods", "*")
    .header("access-control-allow-headers", "*")
    .header("access-control-expose-headers", "*");

  if req.method == Method::OPTIONS {
    return res
      .header("access-control-max-age", "86400")
      .status(crate::StatusCode::NO_CONTENT)
      .body("");
  }

  next.run(req, res, ctx).await
}
