use std::{
    future::Future,
    marker::PhantomData,
    pin::{pin, Pin},
    task::{Context, Poll},
};

use futures::{future::FusedFuture, select, FutureExt};

use crate::value::Notifier;

/// A token which can be used to signal a cancellation request to one or more
/// tasks.
///
/// Tasks can call [`CancellationToken::cancelled()`] in order to
/// obtain a Future which will be resolved when cancellation is requested.
///
/// Cancellation can be requested through the [`CancellationToken::cancel`] method.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    parent_notifier: Option<Notifier>,
    notifier: Notifier,
}

impl CancellationToken {
    /// Creates a new [`CancellationToken`] in the non-cancelled state.
    pub fn new() -> CancellationToken {
        CancellationToken {
            parent_notifier: None,
            notifier: Notifier::new(),
        }
    }

    /// Creates a [`CancellationToken`] which will get cancelled whenever the
    /// current token gets cancelled. Unlike a cloned [`CancellationToken`],
    /// cancelling a child token does not cancel the parent token.
    ///
    /// If the current token is already cancelled, the child token will get
    /// returned in cancelled state.
    pub fn child_token(&self) -> CancellationToken {
        CancellationToken {
            parent_notifier: Some(self.notifier.clone()),
            notifier: Notifier::new(),
        }
    }

    /// Cancel the [`CancellationToken`] and all child tokens which had been
    /// derived from it.
    ///
    /// This will wake up all tasks which are waiting for cancellation.
    ///
    /// Be aware that cancellation is not an atomic operation. It is possible
    /// for another thread running in parallel with a call to `cancel` to first
    /// receive `true` from `is_cancelled` on one child node, and then receive
    /// `false` from `is_cancelled` on another child node. However, once the
    /// call to `cancel` returns, all child nodes have been fully cancelled.
    pub fn cancel(&self) {
        self.notifier.notify();
    }

    /// Returns `true` if the `CancellationToken` is cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.notifier.already_notified()
            || self
                .parent_notifier
                .as_ref()
                .map(Notifier::already_notified)
                .unwrap_or_default()
    }

    /// Returns a [`Future`] that gets fulfilled when cancellation is requested.
    ///
    /// The future will complete immediately if the token is already cancelled
    /// when this method is called.
    ///
    /// # Cancellation safety
    ///
    /// This method is cancel safe.
    pub fn cancelled(&self) -> Cancelled<'_> {
        Cancelled {
            cancelled_owned: CancelledOwned {
                cancellation_token: self.clone(),
            },
            _lifetime: PhantomData,
        }
    }

    /// Returns a [`Future`] that gets fulfilled when cancellation is requested.
    ///
    /// The future will complete immediately if the token is already cancelled
    /// when this method is called.
    ///
    /// The function takes self by value and returns a future that owns the
    /// token.
    ///
    /// # Cancellation safety
    ///
    /// This method is cancel safe.
    pub fn cancelled_owned(self) -> CancelledOwned {
        CancelledOwned {
            cancellation_token: self,
        }
    }

    /// Runs a future to completion and returns its result wrapped inside of an `Option`
    /// unless the [`CancellationToken`] is cancelled. In that case the function returns
    /// `None` and the future gets dropped.
    ///
    /// # Fairness
    ///
    /// Calling this on an already-cancelled token directly returns `None`.
    /// For all subsequent polls, in case of concurrent completion and
    /// cancellation, this is biased towards the future completion.
    ///
    /// # Cancellation safety
    ///
    /// This method is only cancel safe if `fut` is cancel safe.
    pub async fn run_until_cancelled<F>(&self, fut: F) -> Option<F::Output>
    where
        F: Future,
    {
        if self.is_cancelled() {
            None
        } else {
            let mut fut = pin!(fut.fuse());
            select! {
                result = fut => Some(result),
                _ = self.cancelled() => None,
            }
        }
    }

    /// Runs a future to completion and returns its result wrapped inside of an `Option`
    /// unless the [`CancellationToken`] is cancelled. In that case the function returns
    /// `None` and the future gets dropped.
    ///
    /// The function takes self by value and returns a future that owns the token.
    ///
    /// # Fairness
    ///
    /// Calling this on an already-cancelled token directly returns `None`.
    /// For all subsequent polls, in case of concurrent completion and
    /// cancellation, this is biased towards the future completion.
    ///
    /// # Cancellation safety
    ///
    /// This method is only cancel safe if `fut` is cancel safe.
    pub async fn run_until_cancelled_owned<F>(self, fut: F) -> Option<F::Output>
    where
        F: Future,
    {
        self.run_until_cancelled(fut).await
    }
}

/// A Future that is resolved once the corresponding [`CancellationToken`] is cancelled.
#[derive(Debug)]
pub struct Cancelled<'a> {
    cancelled_owned: CancelledOwned,
    _lifetime: PhantomData<&'a ()>,
}

impl<'a> Future for Cancelled<'a> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.cancelled_owned.poll_unpin(cx)
    }
}

impl<'a> FusedFuture for Cancelled<'a> {
    fn is_terminated(&self) -> bool {
        self.cancelled_owned.is_terminated()
    }
}

/// A Future that is resolved once the corresponding [`CancellationToken`]
/// is cancelled.
///
/// This is the counterpart to [`Cancelled`] that takes
/// [`CancellationToken`] by value instead of using a reference.
#[derive(Debug)]
pub struct CancelledOwned {
    cancellation_token: CancellationToken,
}

impl Future for CancelledOwned {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.cancellation_token.notifier.poll_unpin(cx) {
            Poll::Ready(_) => Poll::Ready(()),
            Poll::Pending => {
                if let Some(notifier) = self.cancellation_token.parent_notifier.as_mut() {
                    notifier.poll_unpin(cx)
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

impl FusedFuture for CancelledOwned {
    fn is_terminated(&self) -> bool {
        self.cancellation_token.notifier.is_terminated()
            || self
                .cancellation_token
                .parent_notifier
                .as_ref()
                .map(Notifier::is_terminated)
                .unwrap_or_default()
    }
}
