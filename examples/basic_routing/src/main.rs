/*
  Test with:
  curl http://localhost:8080
  curl http://localhost:8080/foo
  curl http://localhost:8080/bar
  curl http://localhost:8080/fizz/something
*/
use uhttp::Server;
use uhttp::StatusCode;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let mut app = uhttp::router::Router::new_without_context();

  app.get("/foo", |_req, res, _ctx| async move { res.body("foo\n") });

  app.post("/bar", |_req, res, _ctx| async move { res.body("bar\n") });

  app.get("/bar", |_req, res, _ctx| async move { res.body("bar\n") });

  app.get("/fizz/:buzz", |req, res, _ctx| async move {
    let Some(buzz) = req.params.get("buzz") else {
      return res.status(StatusCode::BAD_REQUEST).body("fizz\n");
    };

    res.body(format!("fizz\nParam: {}\n", buzz))
  });

  app.any("/*", |_req, res, _ctx| async move {
    res.body("Not found route")
  });

  Server::builder()
    .handler(app.handler())
    .listen("0.0.0.0:8080")
    .await?;

  Ok(())
}
