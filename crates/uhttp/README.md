# µHTTP

`uhttp` is a lightweight and composible http server and router for building Rust HTTP services

- **Simple:** Inspired by Go's standard library HTTP server. Uses `Read` / `Write` traits for the `Request` / `Response`.

- **Fast:** High performance, multi-threaded implementation built on top of Hyper & Tokio that competes with the fastest Rust HTTP servers.

- **Flexible**: Simple interface that enables many use cases. It can be used directly or to act as a base for frameworks to build on top of.

## Installation

```shell
cargo add uhttp
cargo add uhttp -F json                       # JSON body deserialization
cargo add uhttp -F router                     # Router for URLs
cargo add uhttp -F websocket                  # Support for Websockets
cargo add uhttp -F file_server                # File server that reads from the file system
cargo add uhttp -F full                       # Everything
```

## Usage

### Basic Response

```rust
use uhttp::Server;
use uhttp::StatusCode;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  Server::builder()
    .handler(|_req, res| async move {
      res
        .header("Content-Type", "text/html")
        .body("<body>hello world</body>")
    })
    .listen("0.0.0.0:8080")
    .await?;

  Ok(())
}
```

### Streamed Response

```rust
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::time::sleep;
use uhttp::ResponseBody;
use uhttp::Server;
use uhttp::StatusCode;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  Server::builder()
    .handler(|_req, res| async move {
      // `ResponseBody::chunked` hands back a writer the body is streamed through
      let (body, mut writer) = ResponseBody::chunked(1024);

      tokio::task::spawn(async move {
        for i in 0..10 {
          let _ = writer.write_all(format!("{}", i).as_bytes()).await;
          sleep(Duration::from_millis(1000)).await;
        }
        let _ = writer.shutdown().await;
      });

      res.status(StatusCode::OK).body(body)
    })
    .listen("0.0.0.0:8080")
    .await?;

  Ok(())
}
```

### Static File Server

```rust
use std::path::PathBuf;

use uhttp::Server;
use uhttp::file_server::ETagStrategy;
use uhttp::file_server::FileServerOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  // Change this to the directory where the files live
  let static_files_dir: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static");

  Server::builder()
    .handler(uhttp::file_server::create(FileServerOptions {
      dir: static_files_dir,
      // JIT Compression
      compress: true,
      // ETag to prevent client from making multiple requests
      etag: ETagStrategy::LastModified,
      custom_headers: Default::default(),
      fallback_route: Default::default(),
      fallback_status: Default::default(),
    }))
    .listen("0.0.0.0:8080")
    .await?;

  Ok(())
}
```

### Read / Write JSON Payload

```rust
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
```

### Router

```rust
use uhttp::Server;
use uhttp::StatusCode;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let mut app = uhttp::router::Router::new_without_context();

  app.get("/foo", |_req, res, _ctx| async move {
    res.body("foo\n")
  });

  app.post("/bar", |_req, res, _ctx| async move {
    res.body("bar\n")
  });

  // Example of a URL parameter
  app.get("/fizz/:buzz", |req, res, _ctx| async move {
    let Some(buzz) = req.params.get("buzz") else {
      return res.status(StatusCode::BAD_REQUEST).body("fizz\n");
    };

    res.body(format!("fizz\nParam: {}\n", buzz))
  });

  // Can be used to serve static assets
  app.any("/*", |_req, res, _ctx| async move {
    res.body("Not found route")
  });

  Server::builder()
    .handler(app.handler())
    .listen("0.0.0.0:8080")
    .await?;

  Ok(())
}
```

## Route Context

Route Context is cloned at the start of each request and is used to inject values into request handlers and build context for handlers from middleware.

Examples;

- Inject services that expose database interactions into handlers
- Add validate a JWT and inject access_token information into handlers
- Add logging

```rust
use uhttp::router::Next;

#[derive(Clone)]
struct Context {
  random_string: String,
  reference_string: Arc<String>
}

/// Mutate Context to inject a random string
async fn my_middleware(
  req: uhttp::Request,
  res: uhttp::Response,
  mut ctx: Context,
  next: Next<Context>,
) -> uhttp::HandlerResult {
  ctx.random_string = generate_random_string();
  next.run(req, res, ctx).await
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let ctx = Context {
    random_string: String::default(),
    reference_string: Arc::new(String::from("Reference to a string"))
  };

  let mut app = uhttp::router::Router::new(ctx);

  app.with(my_middleware);

  app.get("/", |_req, res, ctx| async move {
    println!("{}", ctx.reference_string); // Prints "Reference to a string"
    println!("{}", ctx.random_string);    // "<random_string>" each request will get a new random string
    res.body("ok")
  });

  // server.listen
}
```

### HTTP/2

```rust
use uhttp::Server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let cert_path = PathBuf::from(std::env::var("SSL_CERT_PATH").expect("Missing SSL_CERT_PATH env var"));
  let key_path = PathBuf::from(std::env::var("SSL_KEY_PATH").expect("Missing SSL_KEY_PATH env var"));

  // TLS termination is configured through the builder, see the `uhttp` docs for
  // the current TLS options. The handler shape is identical to HTTP/1:
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

```

### Websockets

\_Note: this can also be used from the router

```rust
use std::time::Duration;

use uhttp::Server;
use uhttp::websocket::Websocket;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  Server::builder()
    .handler(|req, res| async move {
      Websocket::upgrade(req, res, |mut socket| async move {
        tokio::task::spawn(async move {
          loop {
            if socket.send("Hello").await.is_err() {
              break;
            };
            tokio::time::sleep(Duration::from_millis(1000)).await;
          }
        });

        Ok(())
      })
    })
    .listen("0.0.0.0:8080")
    .await?;

  Ok(())
}
```

# Benchmarks

- AMD 7950X
- 100GB RAM

## Go

```
Summary:
  Success rate:	100.00%
  Total:	448.1539 ms
  Slowest:	10.6163 ms
  Fastest:	0.0193 ms
  Average:	0.4393 ms
  Requests/sec:	223137.6305

  Total data:	1.14 MiB
  Size/request:	12 B
  Size/sec:	2.55 MiB

Response time histogram:
   0.019 ms [1]     |
   1.079 ms [90055] |■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■
   2.139 ms [8566]  |■■■
   3.198 ms [1106]  |
   4.258 ms [184]   |
   5.318 ms [38]    |
   6.378 ms [13]    |
   7.437 ms [12]    |
   8.497 ms [16]    |
   9.557 ms [5]     |
  10.616 ms [4]     |

Response time distribution:
  10.00% in 0.0869 ms
  25.00% in 0.1345 ms
  50.00% in 0.2533 ms
  75.00% in 0.4971 ms
  90.00% in 1.0753 ms
  95.00% in 1.5735 ms
  99.00% in 2.3279 ms
  99.90% in 4.1377 ms
  99.99% in 8.2329 ms


Details (average, fastest, slowest):
  DNS+dialup:	0.3273 ms, 0.1281 ms, 0.5560 ms
  DNS-lookup:	0.0189 ms, 0.0010 ms, 0.1469 ms

Status code distribution:
  [200] 100000 responses
```

## uHTTP

```
Summary:
  Success rate:	100.00%
  Total:	208.0683 ms
  Slowest:	3.4607 ms
  Fastest:	0.0240 ms
  Average:	0.1963 ms
  Requests/sec:	480611.3014

  Total data:	1.14 MiB
  Size/request:	12 B
  Size/sec:	5.50 MiB

Response time histogram:
  0.024 ms [1]     |
  0.368 ms [95935] |■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■
  0.711 ms [2804]  |
  1.055 ms [857]   |
  1.399 ms [148]   |
  1.742 ms [42]    |
  2.086 ms [45]    |
  2.430 ms [83]    |
  2.773 ms [11]    |
  3.117 ms [21]    |
  3.461 ms [53]    |

Response time distribution:
  10.00% in 0.1046 ms
  25.00% in 0.1325 ms
  50.00% in 0.1700 ms
  75.00% in 0.2183 ms
  90.00% in 0.2819 ms
  95.00% in 0.3441 ms
  99.00% in 0.7954 ms
  99.90% in 2.4065 ms
  99.99% in 3.4094 ms


Details (average, fastest, slowest):
  DNS+dialup:	0.2823 ms, 0.0839 ms, 0.5242 ms
  DNS-lookup:	0.0164 ms, 0.0010 ms, 0.1330 ms

Status code distribution:
  [200] 100000 responses
```
