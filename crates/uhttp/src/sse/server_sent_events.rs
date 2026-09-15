use std::time::Duration;

use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::oneshot;

use super::JsonFrame;
use crate::ResponseBody;

pub struct ServerSentEventsOptions {
  pub max_buffer_size: usize,
  pub heartbeat_duration: Duration,
}

pub struct ServerSentEvents {
  tx: UnboundedSender<ServerSentEventsInternalEvent>,
}

impl ServerSentEvents {
  pub fn new(options: ServerSentEventsOptions) -> (ResponseBody, Self) {
    let ServerSentEventsOptions {
      max_buffer_size,
      heartbeat_duration,
    } = options;

    let (body, mut stream) = ResponseBody::chunked(max_buffer_size);

    let (tx, mut rx) = unbounded_channel::<ServerSentEventsInternalEvent>();

    tokio::spawn({
      let tx = tx.clone();
      async move {
        loop {
          tokio::time::sleep(heartbeat_duration).await;
          if tx.send(ServerSentEventsInternalEvent::HeartBeat).is_err() {
            break;
          }
        }
      }
    });

    tokio::spawn(async move {
      while let Some(event) = rx.recv().await {
        match event {
          ServerSentEventsInternalEvent::HeartBeat => {
            let Ok(frame) = JsonFrame::builder().event("heartbeat").as_frame() else {
              break;
            };
            if stream.write_all(&frame).await.is_err() {
              break;
            }
          }
          ServerSentEventsInternalEvent::Frame {
            value,
            tx_done: done,
          } => {
            let Ok(frame) = JsonFrame::builder().event("event").data(value).as_frame() else {
              let _ = done.send(Err(anyhow::anyhow!("Unable to create frame")));
              continue;
            };
            if stream.write_all(&frame).await.is_err() {
              let _ = done.send(Err(anyhow::anyhow!("Unable to write to socket")));
              continue;
            }
            let _ = done.send(Ok(()));
          }
        }
      }
    });

    (body, ServerSentEvents { tx })
  }

  pub async fn send<S: Serialize>(
    &self,
    message: S,
  ) -> anyhow::Result<()> {
    let value = serde_json::to_value(&message)?;
    let (tx_done, rx_done) = oneshot::channel();

    self
      .tx
      .send(ServerSentEventsInternalEvent::Frame { value, tx_done })?;

    rx_done.await??;

    Ok(())
  }
}

pub struct SseSendError<T>(pub T);

enum ServerSentEventsInternalEvent {
  HeartBeat,
  Frame {
    value: serde_json::Value,
    tx_done: oneshot::Sender<anyhow::Result<()>>,
  },
}
