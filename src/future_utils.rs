use std::pin::Pin;

pub type PinnedFutureBox<T> = Pin<Box<dyn futures::Future<Output = T> + Send>>;

pub trait CallbackWithFuture<T> {
    fn call(self, future: PinnedFutureBox<T>);
}

impl<Function, T> CallbackWithFuture<T> for Function
where
    Function: FnOnce(PinnedFutureBox<T>) + Send,
{
    fn call(self, future: PinnedFutureBox<T>) {
        self(future)
    }
}
