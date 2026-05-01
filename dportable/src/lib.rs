#![warn(missing_docs)]

//! Portable interface for various utilities.
//!
//! Intended targets are:
//!  - native platforms with [tokio](https://docs.rs/tokio/latest/tokio/) async runtime,
//!  - WebAssembly targeted to browsers, including WebWorkers,
//!    under standard single-threaded model.
//!
//! Following features are provided:
//!  - [Mutex] and [RwLock] (using [parking_lot](https://docs.rs/parking_lot/latest/parking_lot/) on native platforms
//!    and [std::cell::RefCell] in WASM).
//!  - asynchronous [spawn] (not requiring [Send] in WASM) and [sleep](time::sleep),
//!  - [yield_now] function,
//!  - [Timeout](time::Timeout) future,
//!  - [dtest](test::dtest) attribute macro to create tests for both
//!    native and WASM targets, also [dtest_configure](test::dtest_configure)
//!    macro to configure tests to run in browser.
//!  - [create_non_sync_send_variant_for_wasm] utility macro for creating
//!    non-[Send] and non-[Sync] variants of traits for use in WASM.
//!  - [AsyncValue](value::AsyncValue) and [Notifier](value::Notifier) for asynchronous value sharing and notification.
//!  - [CancellationToken] for cooperative cancellation of asynchronous tasks.
//!  - [random] function.

pub mod test;

pub mod time;

mod lock;
#[cfg(target_arch = "wasm32")]
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
use js_utils::sleep;
pub use lock::*;

pub mod value;

mod cancellation_token;
pub use cancellation_token::*;

#[cfg(not(target_arch = "wasm32"))]
pub use tokio::{
    spawn,
    task::{yield_now, JoinError, JoinHandle},
};

#[cfg(target_arch = "wasm32")]
/// Yields execution back to the runtime.
///
/// A task yields by awaiting on `yield_now()`, and may resume when that future
/// completes (with no output).
pub async fn yield_now() {
    sleep(Duration::from_millis(0)).await
}

#[cfg(target_arch = "wasm32")]
pub use js_utils::spawn::*;

/// Utility macro for creating non-[Send] and non-[Sync] variants of traits
/// for use in WASM.
///
/// ```
/// use dportable::create_non_sync_send_variant_for_wasm;
///
/// create_non_sync_send_variant_for_wasm! {
///     trait SomeTrait: Send {
///        fn hello(&self);
///     }
/// }
/// ```
pub use dportable_macros::create_non_sync_send_variant_for_wasm;

#[cfg(target_arch = "wasm32")]
/// Returns a floating-point, pseudo-random number in the range 0–1
/// (inclusive of 0, but not 1) with approximately uniform distribution over that range.
pub fn random() -> f64 {
    js_sys::Math::random()
}

#[cfg(not(target_arch = "wasm32"))]
/// Returns a floating-point, pseudo-random number in the range 0–1
/// (inclusive of 0, but not 1) with approximately uniform distribution over that range.
pub fn random() -> f64 {
    rand::random()
}

#[cfg(test)]
mod tests {
    use crate::{
        create_non_sync_send_variant_for_wasm, spawn, test::dtest, time::sleep, yield_now,
        CancellationToken,
    };
    use futures::future::join;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };

    #[dtest]
    async fn test_spawn() {
        let result = spawn(async move { 4 });
        assert_eq!(4, result.await.unwrap());
    }

    #[dtest]
    async fn test_yield_now() {
        let counter = Arc::new(AtomicUsize::new(0));
        let token = CancellationToken::new();

        let counter_task = {
            let counter = counter.clone();
            let token = token.clone();
            spawn(async move {
                while !token.is_cancelled() {
                    counter.fetch_add(1, Ordering::Relaxed);
                    sleep(Duration::from_millis(1)).await;
                }
            })
        };

        let busy_task = {
            let token = token.clone();
            spawn(async move {
                for _ in 0..10_000 {
                    if token.is_cancelled() {
                        break;
                    }

                    let mut x = 0;
                    for _ in 0..100 {
                        x += 1;
                    }

                    let _ = x;
                    yield_now().await;
                }
            })
        };

        // let them run for a bit
        sleep(Duration::from_millis(50)).await;
        token.cancel();

        let _ = join(counter_task, busy_task).await;

        let value = counter.load(Ordering::Relaxed);

        // without proper yielding, this is often 0 or extremely small
        assert!(
            value > 10,
            "counter task made no progress — yield_now likely not working"
        );
    }

    #[dtest]
    async fn test_create_non_sync_send_variant_for_wasm() {
        struct Hello {
            #[cfg(target_arch = "wasm32")]
            _some_reference: std::rc::Rc<()>,

            #[cfg(not(target_arch = "wasm32"))]
            _some_reference: std::sync::Arc<()>,
        }

        create_non_sync_send_variant_for_wasm! {
            trait SomeTrait: Send {
                fn hello(&self);
            }
        }

        impl SomeTrait for Hello {
            fn hello(&self) {
                println!("Hello!");
            }
        }

        let hello = Hello {
            _some_reference: Default::default(),
        };
        spawn(async move {
            hello.hello();
        });
    }
}
