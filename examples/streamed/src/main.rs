/*
  Test with:
    curl -N http://localhost:8080
*/
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::time::sleep;
use uhttp::ResponseBody;
use uhttp::Server;
use uhttp::StatusCode;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  Server::builder()
    .handler(|req, res| async move {
      println!("{}", req.uri.path());

      let (body, mut writer) = ResponseBody::chunked(1024);

      tokio::task::spawn(async move {
        let _ = writer.write_all(b"1\n").await;

        sleep(Duration::from_millis(1000)).await;
        let _ = writer.write_all(b"2\n").await;

        sleep(Duration::from_millis(1000)).await;
        let _ = writer.write_all(b"3\n").await;

        let _ = writer.shutdown().await;
      });

      res.status(StatusCode::OK).body(body)
    })
    .listen("0.0.0.0:8080")
    .await?;

  Ok(())
}
