/*
  Test with:
    curl -H "Content-Type: application/json" -d '{ "message": "Hello World" }' http://localhost:8080
*/
use serde::Deserialize;
use serde::Serialize;
use uhttp::Server;

#[derive(Debug, Serialize, Deserialize)]
pub struct BodyJson {
  pub message: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  Server::builder()
    .handler(|mut req, res| async move {
      // Parse incoming JSON body
      let body = uhttp::body::json::<BodyJson>(&mut req.body).await?;

      // Serialize response body
      let result = serde_json::to_vec(&body)?;

      // Respond with serialized body
      res.body(result)
    })
    .listen("0.0.0.0:8080")
    .await?;

  Ok(())
}
