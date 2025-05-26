use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use pin_project::pin_project;
use tokio::signal::unix::{signal as unix_signal, Signal, SignalKind};
use tokio::sync::Notify;

#[pin_project]
pub(crate) struct SignalManager {
    #[pin]
    sigint: Signal,

    #[pin]
    sigterm: Signal,

    #[pin]
    notify: Arc<Notify>,
}

impl SignalManager {
    pub fn new(notify: Arc<Notify>) -> Self {
        Self {
            notify,
            sigint: unix_signal(SignalKind::interrupt()).unwrap(),
            sigterm: unix_signal(SignalKind::terminate()).unwrap(),
        }
    }
}

impl Future for SignalManager {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        if this.sigint.poll_recv(cx).is_ready() {
            this.notify.notify_waiters();
            return Poll::Ready(());
        };

        if this.sigterm.poll_recv(cx).is_ready() {
            this.notify.notify_waiters();
            return Poll::Ready(());
        };
        Poll::Pending
    }
}
