/*
  Test with:
    curl http://localhost:8080
*/
mod chat_service;

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;
use uhttp::Server;
use uhttp::Websocket;
use uhttp::file_server;
use uhttp::file_server::ETagStrategy;
use uhttp::file_server::FileServerOptions;

use crate::chat_service::ChatService;

static CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[derive(Clone)]
struct Context {
  chat_service: Arc<Mutex<ChatService>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let mut app = uhttp::router::Router::new(Context {
    chat_service: Arc::new(Mutex::new(ChatService::new())),
  });

  app.any(
    "/api/ws",
    move |req, res, Context { chat_service }: Context| async move {
      Websocket::upgrade(req, res, |socket| async move {
        let (mut socket_sender, mut socket_receiver) = socket.split();

        // Send action
        tokio::task::spawn({
          let chat_service = Arc::clone(&chat_service);
          async move {
            let mut chat_service = chat_service.lock().await;

            // Send initial chats as the first message
            let messages = chat_service.get();
            let Ok(msg) = serde_json::to_string_pretty(&messages) else {
              return;
            };

            if socket_sender.send(msg).await.is_err() {
              return;
            };

            // Create subscription for subsequent messages
            let mut rx = chat_service.subsribe();
            drop(chat_service);

            // Emit new messages to current socket
            while let Some((author, message)) = rx.recv().await {
              let msg = serde_json::json!([[author, message]]);
              let Ok(msg) = serde_json::to_string(&msg) else {
                break;
              };

              if socket_sender.send(msg).await.is_err() {
                break;
              };
            }
          }
        });

        // Recieve action
        tokio::task::spawn({
          let chat_service = Arc::clone(&chat_service);
          async move {
            while let Some(msg) = socket_receiver.recv().await? {
              let uhttp::Message::Text(msg) = msg else {
                continue;
              };

              let Ok((author, message)) = serde_json::from_str::<(String, String)>(&msg) else {
                continue;
              };

              let mut chat_service = chat_service.lock().await;
              chat_service.new_message(author, message);
            }

            Ok::<(), anyhow::Error>(())
          }
        });

        Ok(())
      })
    },
  );

  // Serve static files from the "static" directory, and return 404 for missing files
  app.get(
    "/*",
    uhttp::router::without_context(file_server::create(FileServerOptions {
      dir: PathBuf::from(CARGO_MANIFEST_DIR).join("static"),
      compress: false,
      etag: ETagStrategy::LastModified,
      custom_headers: Default::default(),
      fallback_route: Default::default(),
      fallback_status: Default::default(),
    })),
  );

  Server::builder()
    .handler(app.handler())
    .listen("0.0.0.0:8080")
    .await?;

  Ok(())
}
