use crate::types::HyperInfallibleResponse;

pub type HandlerResponse = HyperInfallibleResponse;
pub type HandlerResult = anyhow::Result<HyperInfallibleResponse>;

pub type HandleFunc = Box<
  dyn Send
    + Sync
    + Fn(
      crate::Request,
      crate::Response,
    ) -> std::pin::Pin<
      Box<dyn Send + std::future::Future<Output = crate::Result<HyperInfallibleResponse>>>,
    >,
>;

pub type Result<T> = anyhow::Result<T>;
