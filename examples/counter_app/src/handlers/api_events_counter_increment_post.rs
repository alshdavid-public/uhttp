use uhttp::HandlerResponse;

use crate::context::Context;

pub async fn api_events_counter_increment_post(
  _req: uhttp::Request,
  res: uhttp::Response,
  Context { counter_service }: Context,
) -> uhttp::Result<HandlerResponse> {
  counter_service.increment().await;
  res.status(uhttp::StatusCode::NO_CONTENT).body("")
}
