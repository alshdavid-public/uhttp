use serde::Serialize;
use uhttp::HandlerResponse;

use crate::context::Context;

#[derive(Debug, Serialize)]
struct ApiCounterGetResponse {
  value: isize,
}

// Get current value
pub async fn api_counter_get(
  _req: uhttp::Request,
  res: uhttp::Response,
  Context { counter_service }: Context,
) -> uhttp::Result<HandlerResponse> {
  let counter_value = counter_service.get();

  let json = serde_json::to_vec(&ApiCounterGetResponse {
    value: counter_value,
  })?;

  res.header("Content-Type", "application/json").body(json)
}
