/*
  Test with:
    curl http://localhost:8080
*/
use uhttp::Server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  Server::builder()
    .handler(|_req, res| async move {
      res
        .header("Content-Type", "text/html")
        .body("<body>Hello World!</body>")
    })
    .listen("0.0.0.0:8080")
    .await?;

  Ok(())
}
