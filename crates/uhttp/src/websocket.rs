use std::future::Future;

use anyhow::anyhow;
use futures::SinkExt;
use futures::StreamExt;
use futures::stream::SplitSink;
use futures::stream::SplitStream;
use http::header;
use hyper::upgrade::OnUpgrade;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use tokio_tungstenite::WebSocketStream;
pub use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
pub use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::Role;

type WebsocketIo = WebSocketStream<TokioIo<Upgraded>>;

pub struct Websocket {
  stream: WebsocketIo,
}

impl Websocket {
  pub fn upgrade<F, Fut>(
    mut req: crate::Request,
    res: crate::Response,
    callback: F,
  ) -> crate::Result<crate::HandlerResponse>
  where
    F: 'static + Send + FnOnce(Websocket) -> Fut,
    Fut: 'static + Send + Future<Output = crate::Result<()>>,
  {
    let version = req.headers.get(header::SEC_WEBSOCKET_VERSION);

    if version.map(|version| version.as_bytes()) != Some(b"13") {
      return Err(anyhow!("unsupported websocket version"));
    }

    let key = req
      .headers
      .get(header::SEC_WEBSOCKET_KEY)
      .ok_or_else(|| anyhow!("missing websocket key"))?;

    let accept = derive_accept_key(key.as_bytes());

    let on_upgrade = req
      .extensions
      .remove::<OnUpgrade>()
      .ok_or_else(|| anyhow!("connection cannot be upgraded"))?;

    tokio::spawn(async move {
      let Ok(upgraded) = on_upgrade.await else {
        return;
      };

      let stream =
        WebSocketStream::from_raw_socket(TokioIo::new(upgraded), Role::Server, None).await;

      callback(Websocket { stream }).await.ok();
    });

    res
      .status(crate::StatusCode::SWITCHING_PROTOCOLS)
      .header(header::CONNECTION, "Upgrade")
      .header(header::UPGRADE, "websocket")
      .header(header::SEC_WEBSOCKET_ACCEPT, accept)
      .body("")
  }

  pub async fn send(
    &mut self,
    message: impl Into<Message>,
  ) -> crate::Result<()> {
    self.stream.send(message.into()).await?;
    Ok(())
  }

  pub async fn recv(&mut self) -> crate::Result<Option<Message>> {
    match self.stream.next().await {
      Some(message) => Ok(Some(message?)),
      None => Ok(None),
    }
  }

  pub async fn close(
    &mut self,
    frame: Option<CloseFrame>,
  ) -> crate::Result<()> {
    self.stream.close(frame).await?;
    Ok(())
  }

  pub fn split(self) -> (WebsocketSender, WebsocketReceiver) {
    let (sender, receiver) = self.stream.split();
    (WebsocketSender(sender), WebsocketReceiver(receiver))
  }

  pub fn reunite(
    sender: WebsocketSender,
    receiver: WebsocketReceiver,
  ) -> crate::Result<Self> {
    let stream = sender.0.reunite(receiver.0)?;
    Ok(Self { stream })
  }
}

pub struct WebsocketSender(SplitSink<WebsocketIo, Message>);

impl WebsocketSender {
  pub async fn send(
    &mut self,
    message: impl Into<Message>,
  ) -> crate::Result<()> {
    self.0.send(message.into()).await?;
    Ok(())
  }

  pub async fn close(
    &mut self,
    frame: Option<CloseFrame>,
  ) -> crate::Result<()> {
    self.0.send(Message::Close(frame)).await?;
    Ok(())
  }
}

pub struct WebsocketReceiver(SplitStream<WebsocketIo>);

impl WebsocketReceiver {
  pub async fn recv(&mut self) -> crate::Result<Option<Message>> {
    match self.0.next().await {
      Some(message) => Ok(Some(message?)),
      None => Ok(None),
    }
  }
}
