//! This crate provides a low-level primitive for building Node.js-like event listeners.
//!
//! The 2 primitives are [`Bag`] that is a container for event handlers and [`HandlerId`] that will
//! remove event handler from the bag on drop.
//!
//! Close to real-world usage example:
//! ```rust
//! use event_listener_primitives::{Bag, HandlerId};
//! use std::sync::Arc;
//!
//! #[derive(Default)]
//! struct Handlers {
//!     bar: Bag<'static, dyn Fn() + Send>,
//!     closed: Bag<'static, dyn FnOnce() + Send>,
//! }
//!
//! struct Inner {
//!     handlers: Arc<Handlers>,
//! }
//!
//! impl Drop for Inner {
//!     fn drop(&mut self) {
//!         self.handlers.closed.call_once_simple();
//!     }
//! }
//!
//! #[derive(Clone)]
//! pub struct Foo {
//!     inner: Arc<Inner>,
//! }
//!
//! impl Foo {
//!     pub fn new() -> Self {
//!         let handlers = Arc::<Handlers>::default();
//!
//!         let inner = Arc::new(Inner { handlers });
//!
//!         Self { inner }
//!     }
//!
//!     pub fn do_bar(&self) {
//!         // Do things...
//!
//!         self.inner.handlers.bar.call_simple();
//!     }
//!
//!     pub fn on_bar<F: Fn() + Send + 'static>(&self, callback: F) -> HandlerId {
//!         self.inner.handlers.bar.add(Box::new(callback))
//!     }
//!
//!     pub fn on_closed<F: FnOnce() + Send + 'static>(&self, callback: F) -> HandlerId {
//!         self.inner.handlers.closed.add(Box::new(callback))
//!     }
//! }
//!
//! fn main() {
//!     let foo = Foo::new();
//!     let on_bar_handler_id = foo.on_bar(|| {
//!         println!("On bar");
//!     });
//!     foo
//!         .on_closed(|| {
//!             println!("On closed");
//!         })
//!         .detach();
//!     // This will trigger "bar" callback just fine since its handler ID is not dropped yet
//!     foo.do_bar();
//!     drop(on_bar_handler_id);
//!     // This will not trigger "bar" callback since its handler ID was already dropped
//!     foo.do_bar();
//!     // This will trigger "closed" callback though since we've detached handler OD
//!     drop(foo);
//!
//!     println!("Done");
//! }
//! ```
//!
//! The output will be:
//! ```text
//! On bar
//! On closed
//! Done
//! ```

use std::collections::HashMap;
use std::marker::PhantomData;
use std::mem;
use std::sync::{Arc, Mutex};

/// Handler ID keeps event handler in place, once dropped handler will be removed automatically.
///
/// [`HandlerId::detach()`] can be used if it is not desirable for handler to be removed
/// automatically.
#[must_use = "Handler will be unregistered immediately if not used"]
pub struct HandlerId<'lifetime> {
    callback: Option<Box<dyn FnOnce() + Send + 'lifetime>>,
}

impl<'lifetime> HandlerId<'lifetime> {
    /// Consumes [`HandlerId`] and prevents handler from being removed automatically.
    pub fn detach(mut self) {
        // Remove callback such that it is not called in drop implementation
        self.callback.take();
    }
}

impl<'lifetime> Drop for HandlerId<'lifetime> {
    fn drop(&mut self) {
        if let Some(callback) = self.callback.take() {
            callback();
        }
    }
}

struct Inner<'lifetime, F: ?Sized + Send + 'lifetime> {
    handlers: HashMap<usize, Box<F>>,
    next_index: usize,
    _lifetime: PhantomData<&'lifetime ()>,
}

/// Data structure that holds event handlers
#[derive(Clone)]
pub struct Bag<'lifetime, F: ?Sized + Send + 'lifetime> {
    inner: Arc<Mutex<Inner<'lifetime, F>>>,
    _lifetime: PhantomData<&'lifetime ()>,
}

impl<'lifetime, F: ?Sized + Send + 'lifetime> Default for Bag<'lifetime, F> {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                handlers: HashMap::new(),
                next_index: 0,
                _lifetime: PhantomData::default(),
            })),
            _lifetime: PhantomData::default(),
        }
    }
}

impl<'lifetime, F: ?Sized + Send + 'lifetime> Bag<'lifetime, F> {
    /// Add new event handler to a bag
    pub fn add(&self, callback: Box<F>) -> HandlerId {
        let index;

        {
            let mut inner = self.inner.lock().unwrap();

            index = inner.next_index;
            inner.next_index += 1;

            inner.handlers.insert(index, callback);
        }

        let weak_inner = Arc::downgrade(&self.inner);
        HandlerId {
            callback: Some(Box::new(move || {
                if let Some(inner) = weak_inner.upgrade() {
                    inner.lock().unwrap().handlers.remove(&index);
                }
            })),
        }
    }

    /// Call applicator with each handler and keep handlers in the bag
    pub fn call<A>(&self, applicator: A)
    where
        A: Fn(&Box<F>),
    {
        for callback in self.inner.lock().unwrap().handlers.values() {
            applicator(callback);
        }
    }

    /// Call applicator with each handler and remove handlers from the bag
    pub fn call_once<A>(&self, applicator: A)
    where
        A: Fn(Box<F>),
    {
        for (_, callback) in mem::take(&mut self.inner.lock().unwrap().handlers).into_iter() {
            applicator(callback);
        }
    }
}

impl<'lifetime, F: Fn() + ?Sized + Send> Bag<'lifetime, F> {
    /// Call each handler without arguments and keep handlers in the bag
    pub fn call_simple(&self) {
        for callback in self.inner.lock().unwrap().handlers.values() {
            callback();
        }
    }
}

impl<'lifetime, F: FnOnce() + ?Sized + Send> Bag<'lifetime, F> {
    /// Call each handler without arguments and remove handlers from the bag
    pub fn call_once_simple(&self) {
        for (_, callback) in mem::take(&mut self.inner.lock().unwrap().handlers).into_iter() {
            callback();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn fn_once() {
        let bag = Bag::<dyn FnOnce() + Send>::default();
        let calls = Arc::new(AtomicUsize::new(0));

        {
            let calls = Arc::clone(&calls);
            bag.add(Box::new(move || {
                calls.fetch_add(1, Ordering::SeqCst);
            }))
            .detach();
        }
        {
            let calls = Arc::clone(&calls);
            drop(bag.add(Box::new(move || {
                calls.fetch_add(1, Ordering::SeqCst);
            })));
        }
        bag.call_once(|callback| {
            callback();
        });

        assert_eq!(calls.load(Ordering::SeqCst), 1);

        {
            let calls = Arc::clone(&calls);
            bag.add(Box::new(move || {
                calls.fetch_add(1, Ordering::SeqCst);
            }))
            .detach();
        }
        bag.call_once_simple();

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn fn_regular() {
        let bag = Bag::<dyn Fn() + Send>::default();
        let calls = Arc::new(AtomicUsize::new(0));

        {
            let calls = Arc::clone(&calls);
            bag.add(Box::new(move || {
                calls.fetch_add(1, Ordering::SeqCst);
            }))
            .detach();
        }
        {
            let calls = Arc::clone(&calls);
            drop(bag.add(Box::new(move || {
                calls.fetch_add(1, Ordering::SeqCst);
            })));
        }
        bag.call(|callback| {
            callback();
        });

        assert_eq!(calls.load(Ordering::SeqCst), 1);

        bag.call_simple();

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
