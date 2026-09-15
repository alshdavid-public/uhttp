/*
  Test with:
    websocat ws://localhost:8080
*/
use std::time::Duration;

use uhttp::Server;
use uhttp::Websocket;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  Server::builder()
    .handler(|req, res| async move {
      Websocket::upgrade(req, res, |socket| async move {
        let (mut socket_sender, mut socket_receiver) = socket.split();

        tokio::task::spawn(async move {
          loop {
            if socket_sender.send("Hello").await.is_err() {
              break;
            };
            tokio::time::sleep(Duration::from_millis(1000)).await;
          }
        });

        tokio::task::spawn(async move {
          while let Ok(Some(msg)) = socket_receiver.recv().await {
            println!("GOT: {:?}", msg);
          }
        });

        Ok(())
      })
    })
    .listen("0.0.0.0:8080")
    .await?;

  Ok(())
}
