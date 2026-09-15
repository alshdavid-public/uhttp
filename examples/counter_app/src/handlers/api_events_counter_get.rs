use uhttp::HandlerResponse;
use uhttp::sse::ServerSentEvents;
use uhttp::sse::ServerSentEventsOptions;

use crate::context::Context;

// Event Source that emits when the value is updated
pub async fn api_events_counter_get(
  _req: uhttp::Request,
  res: uhttp::Response,
  Context { counter_service }: Context,
) -> uhttp::Result<HandlerResponse> {
  let (body, events) = ServerSentEvents::new(ServerSentEventsOptions {
    max_buffer_size: 1024,
    heartbeat_duration: std::time::Duration::from_secs(15),
  });

  tokio::task::spawn(async move {
    // Send initial value with subscription
    let _ = events.send(counter_service.get()).await;

    // Listen for updates
    let mut rx = counter_service.subsribe().await;
    while rx.recv().await.is_some() {
      if events.send(counter_service.get()).await.is_err() {
        break;
      }
    }
  });

  res
    .header("Content-Type", "text/event-stream")
    .header("Cache-Control", "no-cache")
    .body(body)
}
