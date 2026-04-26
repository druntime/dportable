#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::CancellationToken;
    use crate::test::{dtest, dtest_configure};
    use crate::time::*;

    dtest_configure!();

    #[dtest]
    async fn test_cancellation_token() {
        let token = CancellationToken::new();
        token.cancel();
        assert!(token.is_cancelled());
        token.cancelled().await;

        let token = CancellationToken::new();
        let result = token.run_until_cancelled(async {
            sleep(Duration::from_millis(10)).await;
            ()
        });
        token.cancel();
        assert!(result.await.is_none());
    }

    #[dtest]
    async fn test_child_token() {
        let token = CancellationToken::new();
        let child_token = token.child_token();
        token.cancel();
        assert!(child_token.is_cancelled());

        let token = CancellationToken::new();
        let child_token = token.child_token();
        child_token.cancel();
        assert!(!token.is_cancelled());
        assert!(child_token.is_cancelled());

        let token = CancellationToken::new();
        let child_token = token.child_token();
        let result = token.run_until_cancelled(async {
            sleep(Duration::from_millis(10)).await;
            ()
        });
        let child_result = child_token.run_until_cancelled(async {
            sleep(Duration::from_millis(10)).await;
            ()
        });
        token.cancel();
        assert!(result.await.is_none());
        assert!(child_result.await.is_none());

        let token = CancellationToken::new();
        let child_token = token.child_token();
        let result = token.run_until_cancelled(async {
            sleep(Duration::from_millis(10)).await;
            ()
        });
        let child_result = child_token.run_until_cancelled(async {
            sleep(Duration::from_millis(10)).await;
            ()
        });
        child_token.cancel();
        assert!(result.await.is_some());
        assert!(child_result.await.is_none());
    }
}
