use crate::HandlerId;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tinyvec::TinyVec;

struct Inner<F: Send + Sync + 'static> {
    handlers: HashMap<usize, Arc<Box<F>>>,
    next_index: usize,
}

/// Data structure that holds `Fn()` event handlers
pub struct Bag<F: Send + Sync + 'static> {
    inner: Arc<Mutex<Inner<F>>>,
}

impl<F: Send + Sync + 'static> Clone for Bag<F> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<F: Send + Sync + 'static> Default for Bag<F> {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                handlers: HashMap::new(),
                next_index: 0,
            })),
        }
    }
}

impl<F: Send + Sync + 'static> Bag<F> {
    /// Add new event handler to a bag
    pub fn add(&self, callback: F) -> HandlerId {
        self.add_boxed_arc(Arc::new(Box::new(callback)))
    }

    /// Add new event handler to a bag that is already `Arc<Box<Fn()>>`
    pub fn add_boxed_arc(&self, callback: Arc<Box<F>>) -> HandlerId {
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

    /// Call applicator with each handler and keep handlers in the bag
    pub fn call<A>(&self, applicator: A)
    where
        A: Fn(&Box<F>),
    {
        // We collect handlers first in order to avoid holding lock while calling handlers
        let handlers = self
            .inner
            .lock()
            .handlers
            .values()
            .map(|handler| Some(Arc::clone(handler)))
            .collect::<TinyVec<[Option<Arc<Box<F>>>; 10]>>();
        for handler in handlers.iter() {
            applicator(handler.as_ref().unwrap());
        }
    }
}

impl<F: Fn() + Send + Sync + 'static> Bag<F> {
    /// Call each handler without arguments and keep handlers in the bag
    pub fn call_simple(&self) {
        self.call(|handler| handler())
    }
}
