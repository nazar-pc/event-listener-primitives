use crate::HandlerId;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::mem;
use std::sync::Arc;

struct Inner {
    handlers: HashMap<usize, Box<dyn FnOnce() + Send + 'static>>,
    next_index: usize,
}

/// Data structure that holds `FnOnce()` event handlers
pub struct BagOnce {
    inner: Arc<Mutex<Inner>>,
}

impl Clone for BagOnce {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Default for BagOnce {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                handlers: HashMap::new(),
                next_index: 0,
            })),
        }
    }
}

impl BagOnce {
    /// Add new event handler to a bag
    pub fn add<F>(&self, callback: F) -> HandlerId
    where
        F: FnOnce() + Send + 'static,
    {
        let index;

        {
            let mut inner = self.inner.lock();

            index = inner.next_index;
            inner.next_index += 1;

            inner.handlers.insert(index, Box::new(callback));
        }

        HandlerId::new({
            let weak_inner = Arc::downgrade(&self.inner);

            move || {
                if let Some(inner) = weak_inner.upgrade() {
                    inner.lock().handlers.remove(&index);
                }
            }
        })
    }

    /// Call applicator with each handler and remove handlers from the bag
    pub fn call<A>(&self, applicator: A)
    where
        A: Fn(Box<dyn FnOnce() + Send + 'static>),
    {
        // We collect handlers first in order to avoid holding lock while calling handlers
        let handlers = mem::take(&mut self.inner.lock().handlers);
        for (_, handler) in handlers.into_iter() {
            applicator(handler);
        }
    }

    /// Call each handler without arguments and remove handlers from the bag
    pub fn call_simple(&self) {
        self.call(|handler| handler())
    }
}
