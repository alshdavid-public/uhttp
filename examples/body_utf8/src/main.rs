/*
  Test with:
    curl -H "Content-Type: text/plain" -d 'Hello From Client' http://localhost:8080
*/
use uhttp::Server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  Server::builder()
    .handler(|mut req, res| async move {
      println!("{}", req.uri);

      let body = uhttp::body::utf8(&mut req.body).await?;
      println!("{}", body);

      res.body("Ok\n")
    })
    .listen("0.0.0.0:8080")
    .await?;

  Ok(())
}
