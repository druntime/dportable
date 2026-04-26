pub use tokio_util::sync::CancellationToken;

/// A Future that is resolved once the corresponding [`CancellationToken`] is cancelled.
pub type Cancelled<'a> = tokio_util::sync::WaitForCancellationFuture<'a>;

/// A Future that is resolved once the corresponding [`CancellationToken`]
/// is cancelled.
///
/// This is the counterpart to [`Cancelled`] that takes
/// [`CancellationToken`] by value instead of using a reference.
pub type CancelledOwned = tokio_util::sync::WaitForCancellationFutureOwned;
