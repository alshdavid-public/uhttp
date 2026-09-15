use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use hyper::service::service_fn;
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::TokioIo;
use hyper_util::server::conn::auto;
use hyper_util::server::graceful::GracefulShutdown;
use tokio::net::TcpListener;
use tokio::net::ToSocketAddrs;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::types::HttpBoxBody;
use crate::types::HttpFull;
use crate::types::HyperBytes;
use crate::types::HyperInfallibleResponse;
use crate::types::HyperResponse;

fn handle_error(error: anyhow::Error) -> HyperInfallibleResponse {
  let content = HyperBytes::from(format!("{}", error));
  let body = HttpBoxBody::new(HttpFull::new(content));
  let response = HyperResponse::builder().status(500).body(body);

  let Ok(response) = response else { todo!() };

  response
}

pub type ServerHandleFunc = Arc<
  dyn 'static
    + Send
    + Sync
    + Fn(
      crate::Request,
      crate::Response,
    )
      -> Pin<Box<dyn 'static + Send + Future<Output = crate::Result<HyperInfallibleResponse>>>>,
>;

pub struct NoHandler;

pub struct Server {
  handler: ServerHandleFunc,
  listener: TcpListener,
  local_addr: SocketAddr,
  cancellation_token: CancellationToken,
}

impl Server {
  pub fn builder() -> ServerBuilder<NoHandler> {
    ServerBuilder {
      handler: NoHandler,
      cancellation_token: None,
    }
  }

  pub fn local_addr(&self) -> SocketAddr {
    self.local_addr
  }

  pub fn cancellation_token(&self) -> &CancellationToken {
    &self.cancellation_token
  }

  pub fn shutdown(&self) {
    self.cancellation_token.cancel();
  }

  pub async fn serve(self) -> anyhow::Result<()> {
    let service_builder = auto::Builder::new(TokioExecutor::new());
    let graceful = GracefulShutdown::new();

    loop {
      let accepted = tokio::select! {
        accepted = self.listener.accept() => accepted,
        _ = self.cancellation_token.cancelled() => break,
      };

      let Ok((stream, _)) = accepted else {
        continue;
      };
      let io = TokioIo::new(stream);
      let handler = Arc::clone(&self.handler);

      let service_handler = service_fn(move |req| {
        let fut = handler(crate::Request::from(req), crate::Response::new());

        async move {
          let handler_response = match fut.await {
            Ok(handler_response) => handler_response,
            Err(handler_error) => handle_error(handler_error),
          };

          Ok::<HyperInfallibleResponse, anyhow::Error>(handler_response)
        }
      });

      let connection = service_builder
        .serve_connection_with_upgrades(io, service_handler)
        .into_owned();

      let connection = graceful.watch(connection);

      tokio::task::spawn(async move {
        connection.await.ok();
      });
    }

    drop(self.listener);
    graceful.shutdown().await;

    Ok(())
  }

  pub fn spawn(self) -> ServerHandle {
    let local_addr = self.local_addr;
    let cancellation_token = self.cancellation_token.clone();

    ServerHandle {
      local_addr,
      cancellation_token,
      task: tokio::task::spawn(self.serve()),
    }
  }
}

pub struct ServerHandle {
  local_addr: SocketAddr,
  cancellation_token: CancellationToken,
  task: JoinHandle<anyhow::Result<()>>,
}

impl ServerHandle {
  pub fn local_addr(&self) -> SocketAddr {
    self.local_addr
  }

  pub fn cancellation_token(&self) -> &CancellationToken {
    &self.cancellation_token
  }

  pub fn shutdown(&self) {
    self.cancellation_token.cancel();
  }

  pub async fn join(self) -> anyhow::Result<()> {
    self.task.await?
  }
}

pub struct ServerBuilder<H = NoHandler> {
  handler: H,
  cancellation_token: Option<CancellationToken>,
}

impl<H> ServerBuilder<H> {
  pub fn cancellation_token(
    mut self,
    cancellation_token: CancellationToken,
  ) -> Self {
    self.cancellation_token = Some(cancellation_token);
    self
  }
}

impl ServerBuilder<NoHandler> {
  pub fn handler<F, Fut>(
    self,
    handler: F,
  ) -> ServerBuilder<ServerHandleFunc>
  where
    F: 'static + Send + Sync + Fn(crate::Request, crate::Response) -> Fut,
    Fut: 'static + Send + Future<Output = crate::Result<HyperInfallibleResponse>>,
  {
    ServerBuilder {
      handler: Arc::new(move |req, res| Box::pin(handler(req, res))),
      cancellation_token: self.cancellation_token,
    }
  }
}

impl ServerBuilder<ServerHandleFunc> {
  pub async fn bind<A>(
    self,
    addr: A,
  ) -> anyhow::Result<Server>
  where
    A: ToSocketAddrs,
  {
    let listener = TcpListener::bind(&addr).await?;
    let local_addr = listener.local_addr()?;

    Ok(Server {
      handler: self.handler,
      listener,
      local_addr,
      cancellation_token: self.cancellation_token.unwrap_or_default(),
    })
  }

  pub async fn listen<A>(
    self,
    addr: A,
  ) -> anyhow::Result<()>
  where
    A: ToSocketAddrs,
  {
    self.bind(addr).await?.serve().await
  }
}
