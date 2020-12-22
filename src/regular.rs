use crate::HandlerId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tinyvec::TinyVec;

type WrappedHandler = Arc<Box<dyn Fn() + Send + Sync + 'static>>;

struct Inner {
    handlers: HashMap<usize, WrappedHandler>,
    next_index: usize,
}

/// Data structure that holds `Fn()` event handlers
pub struct Bag {
    inner: Arc<Mutex<Inner>>,
}

impl Clone for Bag {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Default for Bag {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                handlers: HashMap::new(),
                next_index: 0,
            })),
        }
    }
}

impl Bag {
    /// Add new event handler to a bag
    pub fn add<F>(&self, callback: F) -> HandlerId
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.add_boxed_arc(Arc::new(Box::new(callback)))
    }

    /// Add new event handler to a bag that is already `Arc<Box<Fn()>>`
    pub fn add_boxed_arc(&self, callback: WrappedHandler) -> HandlerId {
        let index;

        {
            let mut inner = self.inner.lock().unwrap();

            index = inner.next_index;
            inner.next_index += 1;

            inner.handlers.insert(index, callback);
        }

        HandlerId::new({
            let weak_inner = Arc::downgrade(&self.inner);

            move || {
                if let Some(inner) = weak_inner.upgrade() {
                    inner.lock().unwrap().handlers.remove(&index);
                }
            }
        })
    }

    /// Call applicator with each handler and keep handlers in the bag
    pub fn call<A>(&self, applicator: A)
    where
        A: Fn(&Box<dyn Fn() + Send + Sync + 'static>),
    {
        // We collect handlers first in order to avoid holding lock while calling handlers
        let handlers = self
            .inner
            .lock()
            .unwrap()
            .handlers
            .values()
            .map(|handler| Some(Arc::clone(handler)))
            .collect::<TinyVec<[Option<WrappedHandler>; 10]>>();
        for handler in handlers.iter() {
            applicator(handler.as_ref().unwrap());
        }
    }

    /// Call each handler without arguments and keep handlers in the bag
    pub fn call_simple(&self) {
        self.call(|handler| handler())
    }
}
