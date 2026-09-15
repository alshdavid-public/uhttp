use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

use http::Method;
use path_tree::PathTree;
use percent_encoding::percent_decode_str;

use crate::HandlerResponse;
use crate::Request;
use crate::Response;

pub type RouterFuture =
  Pin<Box<dyn 'static + Send + Future<Output = crate::Result<HandlerResponse>>>>;

pub(super) type RouterHandleFuncInner<Context> =
  Arc<dyn 'static + Send + Sync + Fn(Request, Response, Context) -> RouterFuture>;

pub(super) type RouterMiddlewareFuncInner<Context> =
  Arc<dyn 'static + Send + Sync + Fn(Request, Response, Context, Next<Context>) -> RouterFuture>;

pub type RouterHandleFunc<Context> =
  Box<dyn 'static + Send + Sync + Fn(Request, Response, Context) -> RouterFuture>;

pub type RouterMiddlewareFunc<Context> =
  Box<dyn 'static + Send + Sync + Fn(Request, Response, Context, Next<Context>) -> RouterFuture>;

pub struct Next<Context> {
  middleware: std::vec::IntoIter<RouterMiddlewareFuncInner<Context>>,
  handler: RouterHandleFuncInner<Context>,
}

impl<Context> Next<Context> {
  pub fn run(
    mut self,
    req: Request,
    res: Response,
    ctx: Context,
  ) -> RouterFuture {
    match self.middleware.next() {
      Some(middleware) => middleware(req, res, ctx, self),
      None => (self.handler)(req, res, ctx),
    }
  }
}

pub(super) type PathTreeRoute<T> = (Vec<RouterMiddlewareFuncInner<T>>, RouterHandleFuncInner<T>);

pub struct Router<T>
where
  T: Clone + Send + Sync + 'static,
{
  base_path: String,
  middleware: Vec<RouterMiddlewareFuncInner<T>>,
  any_routes: Rc<RefCell<PathTree<PathTreeRoute<T>>>>,
  get_routes: Rc<RefCell<PathTree<PathTreeRoute<T>>>>,
  post_routes: Rc<RefCell<PathTree<PathTreeRoute<T>>>>,
  put_routes: Rc<RefCell<PathTree<PathTreeRoute<T>>>>,
  patch_routes: Rc<RefCell<PathTree<PathTreeRoute<T>>>>,
  delete_routes: Rc<RefCell<PathTree<PathTreeRoute<T>>>>,
  context: T,
}

impl Router<()> {
  pub fn new_without_context() -> Router<()> {
    Self::new(())
  }
}

impl<T: Clone + Send + Sync + 'static> Router<T> {
  pub fn new(context: T) -> Self {
    Self {
      base_path: String::new(),
      middleware: Vec::new(),
      any_routes: Rc::new(RefCell::new(PathTree::new())),
      get_routes: Rc::new(RefCell::new(PathTree::new())),
      post_routes: Rc::new(RefCell::new(PathTree::new())),
      put_routes: Rc::new(RefCell::new(PathTree::new())),
      patch_routes: Rc::new(RefCell::new(PathTree::new())),
      delete_routes: Rc::new(RefCell::new(PathTree::new())),
      context,
    }
  }

  pub fn extend(
    source: &Self,
    base_path: &str,
  ) -> Router<T> {
    Router {
      base_path: base_path.to_string(),
      middleware: source.middleware.clone(),
      any_routes: Rc::clone(&source.any_routes),
      get_routes: Rc::clone(&source.get_routes),
      post_routes: Rc::clone(&source.post_routes),
      put_routes: Rc::clone(&source.put_routes),
      patch_routes: Rc::clone(&source.patch_routes),
      delete_routes: Rc::clone(&source.delete_routes),
      context: source.context.clone(),
    }
  }

  pub fn with<F, Fut>(
    &mut self,
    middleware: F,
  ) -> Router<T>
  where
    F: 'static + Send + Sync + Fn(Request, Response, T, Next<T>) -> Fut,
    Fut: 'static + Send + Future<Output = crate::Result<HandlerResponse>>,
  {
    let middleware: RouterMiddlewareFuncInner<T> =
      Arc::new(move |req, res, ctx, next| Box::pin(middleware(req, res, ctx, next)));

    let mut current_middleware = self.middleware.clone();
    current_middleware.push(middleware);
    Router {
      base_path: self.base_path.clone(),
      middleware: current_middleware,
      any_routes: Rc::clone(&self.any_routes),
      get_routes: Rc::clone(&self.get_routes),
      post_routes: Rc::clone(&self.post_routes),
      put_routes: Rc::clone(&self.put_routes),
      patch_routes: Rc::clone(&self.patch_routes),
      delete_routes: Rc::clone(&self.delete_routes),
      context: self.context.clone(),
    }
  }

  pub fn get<F, Fut>(
    &mut self,
    route: &str,
    handler: F,
  ) where
    F: 'static + Send + Sync + Fn(Request, Response, T) -> Fut,
    Fut: 'static + Send + Future<Output = crate::Result<crate::HandlerResponse>>,
  {
    let _ = self.get_routes.borrow_mut().insert(
      &self.route(route),
      (
        self.middleware.clone(),
        Arc::new(move |req, res, ctx| Box::pin(handler(req, res, ctx))),
      ),
    );
  }

  pub fn post<F, Fut>(
    &mut self,
    route: &str,
    handler: F,
  ) where
    F: 'static + Send + Sync + Fn(Request, Response, T) -> Fut,
    Fut: 'static + Send + Future<Output = crate::Result<crate::HandlerResponse>>,
  {
    let _ = self.post_routes.borrow_mut().insert(
      &self.route(route),
      (
        self.middleware.clone(),
        Arc::new(move |req, res, ctx| Box::pin(handler(req, res, ctx))),
      ),
    );
  }

  pub fn put<F, Fut>(
    &mut self,
    route: &str,
    handler: F,
  ) where
    F: 'static + Send + Sync + Fn(Request, Response, T) -> Fut,
    Fut: 'static + Send + Future<Output = crate::Result<crate::HandlerResponse>>,
  {
    let _ = self.put_routes.borrow_mut().insert(
      &self.route(route),
      (
        self.middleware.clone(),
        Arc::new(move |req, res, ctx| Box::pin(handler(req, res, ctx))),
      ),
    );
  }

  pub fn patch<F, Fut>(
    &mut self,
    route: &str,
    handler: F,
  ) where
    F: 'static + Send + Sync + Fn(Request, Response, T) -> Fut,
    Fut: 'static + Send + Future<Output = crate::Result<crate::HandlerResponse>>,
  {
    let _ = self.patch_routes.borrow_mut().insert(
      &self.route(route),
      (
        self.middleware.clone(),
        Arc::new(move |req, res, ctx| Box::pin(handler(req, res, ctx))),
      ),
    );
  }

  pub fn delete<F, Fut>(
    &mut self,
    route: &str,
    handler: F,
  ) where
    F: 'static + Send + Sync + Fn(Request, Response, T) -> Fut,
    Fut: 'static + Send + Future<Output = crate::Result<crate::HandlerResponse>>,
  {
    let _ = self.delete_routes.borrow_mut().insert(
      &self.route(route),
      (
        self.middleware.clone(),
        Arc::new(move |req, res, ctx| Box::pin(handler(req, res, ctx))),
      ),
    );
  }

  pub fn any<F, Fut>(
    &mut self,
    route: &str,
    handler: F,
  ) where
    F: 'static + Send + Sync + Fn(Request, Response, T) -> Fut,
    Fut: 'static + Send + Future<Output = crate::Result<crate::HandlerResponse>>,
  {
    let handler: RouterHandleFuncInner<T> =
      Arc::new(move |req, res, ctx| Box::pin(handler(req, res, ctx)));

    let _ = self.get_routes.borrow_mut().insert(
      &self.route(route),
      (self.middleware.clone(), Arc::clone(&handler)),
    );

    let _ = self.post_routes.borrow_mut().insert(
      &self.route(route),
      (self.middleware.clone(), Arc::clone(&handler)),
    );

    let _ = self.put_routes.borrow_mut().insert(
      &self.route(route),
      (self.middleware.clone(), Arc::clone(&handler)),
    );

    let _ = self.patch_routes.borrow_mut().insert(
      &self.route(route),
      (self.middleware.clone(), Arc::clone(&handler)),
    );

    let _ = self.delete_routes.borrow_mut().insert(
      &self.route(route),
      (self.middleware.clone(), Arc::clone(&handler)),
    );

    let _ = self
      .any_routes
      .borrow_mut()
      .insert(&self.route(route), (self.middleware.clone(), handler));
  }

  pub fn handler(&self) -> crate::HandleFunc {
    let middleware = Arc::new(self.middleware.clone());
    let any_routes = Arc::new(self.any_routes.borrow().clone());
    let get_routes = Arc::new(self.get_routes.borrow().clone());
    let post_routes = Arc::new(self.post_routes.borrow().clone());
    let put_routes = Arc::new(self.put_routes.borrow().clone());
    let patch_routes = Arc::new(self.patch_routes.borrow().clone());
    let delete_routes = Arc::new(self.delete_routes.borrow().clone());
    let context = self.context.clone();

    Box::new(move |mut req, res| {
      let middleware = middleware.clone();
      let any_routes = any_routes.clone();
      let get_routes = get_routes.clone();
      let post_routes = post_routes.clone();
      let put_routes = put_routes.clone();
      let patch_routes = patch_routes.clone();
      let delete_routes = delete_routes.clone();
      let context = context.clone();

      Box::pin(async move {
        let path = req.uri.path().to_string();

        let routes = match req.method {
          Method::GET => get_routes,
          Method::POST => post_routes,
          Method::PUT => put_routes,
          Method::PATCH => patch_routes,
          Method::DELETE => delete_routes,
          _ => Arc::clone(&any_routes),
        };

        let Some(((route_middleware, handler), params)) = routes.find(&path) else {
          return Next {
            middleware: middleware.as_ref().clone().into_iter(),
            handler: Arc::new(|_req, res: Response, _ctx| {
              Box::pin(async move { res.status(crate::StatusCode::NOT_FOUND).body("") })
            }),
          }
          .run(req, res, context)
          .await;
        };

        for (key, value) in params.params() {
          req.params.insert(
            key.to_string(),
            percent_decode_str(value).decode_utf8_lossy().to_string(),
          );
        }

        Next {
          middleware: route_middleware.clone().into_iter(),
          handler: Arc::clone(handler),
        }
        .run(req, res, context)
        .await
      })
    })
  }

  fn route(
    &self,
    mut target: &str,
  ) -> String {
    if !self.base_path.is_empty() && target == "/" {
      target = ""
    }
    format!("{}{}", self.base_path, target)
  }
}

pub fn without_context<T: 'static + Clone + Send + Sync>(
  handler: crate::HandleFunc
) -> crate::router::RouterHandleFunc<T> {
  Box::new(move |req, res, _ctx| handler(req, res))
}
