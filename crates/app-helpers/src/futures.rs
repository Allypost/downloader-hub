use futures::executor;
use tokio::runtime::Handle;

pub fn run_async<T>(fut: impl std::future::Future<Output = T>) -> T {
    let handle = Handle::current();
    let _ = handle.enter();
    executor::block_on(fut)
}
