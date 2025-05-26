use std::future::Future;
use std::task::Poll;

use pin_project::pin_project;
use tokio::time::Instant;

use crate::runtime::metrics::get_task_duration_histogram;

#[pin_project]
pub struct Timer<T> {
    #[pin]
    inner: T,
    start: Option<Instant>,
}

impl<T> Timer<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, start: None }
    }
}

impl<T> Future for Timer<T>
where
    T: Future,
{
    type Output = T::Output;
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let start = this.start.get_or_insert_with(Instant::now);

        match this.inner.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(v) => {
                get_task_duration_histogram().record(start.elapsed().as_secs(), &[]);

                Poll::Ready(v)
            }
        }
    }
}
