/*
  Test with:
    curl http://localhost:8080/?hello=world
*/
use std::collections::HashMap;

use uhttp::Server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  Server::builder()
    .handler(|req, res| async move {
      let query = req.query::<HashMap<String, String>>()?;

      res
        .header("Content-Type", "text/html")
        .body(format!("<body>{:?}</body>", query))
    })
    .listen("0.0.0.0:8080")
    .await?;

  Ok(())
}
