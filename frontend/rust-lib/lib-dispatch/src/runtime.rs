use futures_core::ready;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{fmt, io, thread};

use tokio::runtime;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub struct AFPluginRuntime {
  inner: Runtime,
  #[cfg(any(target_arch = "wasm32", feature = "local_set"))]
  local_set: tokio::task::LocalSet,
  #[cfg(any(target_arch = "wasm32", feature = "local_set"))]
  local_set_handler: LocalSetHandle,
}

impl Display for AFPluginRuntime {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    if cfg!(any(target_arch = "wasm32", feature = "local_set")) {
      write!(f, "Runtime(current_thread)")
    } else {
      write!(f, "Runtime(multi_thread)")
    }
  }
}

impl AFPluginRuntime {
  pub fn new() -> io::Result<Self> {
    let inner = default_tokio_runtime()?;
    #[cfg(any(target_arch = "wasm32", feature = "local_set"))]
    {
      let local_set = tokio::task::LocalSet::new();
      let (tx, rx) = mpsc::unbounded_channel();
      let local_set_handler = LocalSetHandle::new(tx);
      // let runner = LocalSetRunner { rx };
      // local_set.block_on(&inner, runner);
      Ok(Self {
        inner,
        local_set,
        local_set_handler,
      })
    }

    #[cfg(all(not(target_arch = "wasm32"), not(feature = "local_set")))]
    {
      Ok(Self { inner })
    }
  }

  #[cfg(any(target_arch = "wasm32", feature = "local_set"))]
  #[track_caller]
  pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
  where
    F: Future + 'static,
  {
    self.local_set.spawn_local(future)
  }

  #[cfg(all(not(target_arch = "wasm32"), not(feature = "local_set")))]
  #[track_caller]
  pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
  where
    F: Future + Send + 'static,
    <F as Future>::Output: Send + 'static,
  {
    self.inner.spawn(future)
  }

  #[cfg(any(target_arch = "wasm32", feature = "local_set"))]
  pub async fn run_until<F>(&self, future: F) -> F::Output
  where
    F: Future,
  {
    self.local_set.run_until(future).await
  }

  #[cfg(all(not(target_arch = "wasm32"), not(feature = "local_set")))]
  pub async fn run_until<F>(&self, future: F) -> F::Output
  where
    F: Future,
  {
    future.await
  }

  #[cfg(any(target_arch = "wasm32", feature = "local_set"))]
  #[track_caller]
  pub fn block_on<F>(&self, f: F) -> F::Output
  where
    F: Future,
  {
    self.local_set.block_on(&self.inner, f)
  }

  #[cfg(all(not(target_arch = "wasm32"), not(feature = "local_set")))]
  #[track_caller]
  pub fn block_on<F>(&self, f: F) -> F::Output
  where
    F: Future,
  {
    self.inner.block_on(f)
  }
}

#[cfg(any(target_arch = "wasm32", feature = "local_set"))]
pub fn default_tokio_runtime() -> io::Result<Runtime> {
  runtime::Builder::new_current_thread()
    .thread_name("dispatch-current")
    .worker_threads(6)
    .build()
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "local_set")))]
pub fn default_tokio_runtime() -> io::Result<Runtime> {
  runtime::Builder::new_multi_thread()
    .thread_name("dispatch-multi")
    .enable_io()
    .enable_time()
    .on_thread_start(move || {
      tracing::trace!(
        "{:?} thread started: thread_id= {}",
        std::thread::current(),
        thread_id::get()
      );
    })
    .on_thread_stop(move || {
      tracing::trace!(
        "{:?} thread stopping: thread_id= {}",
        std::thread::current(),
        thread_id::get(),
      );
    })
    .build()
}

#[derive(Debug, Clone)]
pub struct LocalSetHandle {
  tx: mpsc::UnboundedSender<LocalSetCommand>,
}

impl LocalSetHandle {
  pub(crate) fn new(tx: mpsc::UnboundedSender<LocalSetCommand>) -> Self {
    Self { tx }
  }

  pub fn spawn<Fut>(&self, future: Fut) -> bool
  where
    Fut: Future<Output = ()> + 'static,
  {
    self
      .tx
      .send(LocalSetCommand::Execute(Box::pin(future)))
      .is_ok()
  }

  pub fn spawn_fn<F>(&self, f: F) -> bool
  where
    F: FnOnce() + 'static,
  {
    self.spawn(async { f() })
  }

  pub fn stop(&self) -> bool {
    self.tx.send(LocalSetCommand::Stop).is_ok()
  }
}

pub(crate) enum LocalSetCommand {
  Stop,
  Execute(Pin<Box<dyn Future<Output = ()>>>),
}

struct LocalSetRunner {
  rx: mpsc::UnboundedReceiver<LocalSetCommand>,
}

impl Future for LocalSetRunner {
  type Output = ();

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    loop {
      match ready!(self.rx.poll_recv(cx)) {
        None => return Poll::Ready(()),

        Some(item) => match item {
          LocalSetCommand::Stop => {
            return Poll::Ready(());
          },
          LocalSetCommand::Execute(fut) => {
            tokio::task::spawn_local(fut);
          },
        },
      }
    }
  }
}
