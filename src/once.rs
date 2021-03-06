use crate::HandlerId;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::{fmt, mem};

struct Inner<F: Send + 'static> {
    handlers: HashMap<usize, F>,
    next_index: usize,
}

/// Data structure that holds `FnOnce()` event handlers
pub struct BagOnce<F: Send + 'static> {
    inner: Arc<Mutex<Inner<F>>>,
}

impl<F: Send + Sync + 'static> fmt::Debug for BagOnce<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BagOnce").finish()
    }
}

impl<F: Send + 'static> Clone for BagOnce<F> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<F: Send + 'static> Default for BagOnce<F> {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                handlers: HashMap::new(),
                next_index: 0,
            })),
        }
    }
}

impl<F: Send + 'static> BagOnce<F> {
    /// Add new event handler to a bag
    pub fn add(&self, callback: F) -> HandlerId {
        let index;

        {
            let mut inner = self.inner.lock();

            index = inner.next_index;
            inner.next_index += 1;

            inner.handlers.insert(index, callback);
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
        A: Fn(F),
    {
        // We collect handlers first in order to avoid holding lock while calling handlers
        let handlers = mem::take(&mut self.inner.lock().handlers);
        for (_, handler) in handlers {
            applicator(handler);
        }
    }
}

impl<F: FnOnce() + Send + 'static> BagOnce<F> {
    /// Call each handler without arguments and remove handlers from the bag
    pub fn call_simple(&self) {
        self.call(|handler| handler())
    }
}
