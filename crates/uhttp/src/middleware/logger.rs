use std::time::Instant;

use crate::router::Next;

pub async fn logger_default<T>(
  req: crate::Request,
  res: crate::Response,
  ctx: T,
  next: Next<T>,
) -> crate::HandlerResult {
  let method = req.method.clone();
  let path = match req.uri.path_and_query() {
    Some(path) => path.as_str().to_string(),
    None => String::new(),
  };

  let start = Instant::now();
  let result = next.run(req, res, ctx).await;
  let elapsed = start.elapsed();

  let status = match &result {
    Ok(res) => res.status().as_u16(),
    Err(_) => 500,
  };

  println!("[{}] {} {} {:?}", method, path, status, elapsed);
  result
}

pub struct LoggerOptions {
  // todo
}

pub fn logger<T: 'static + Clone + Send + Sync>(
  _options: LoggerOptions
) -> crate::router::RouterMiddlewareFunc<T> {
  Box::new(|req, res, ctx, next| Box::pin(logger_default(req, res, ctx, next)))
}
