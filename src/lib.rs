#![warn(rust_2018_idioms, missing_debug_implementations, missing_docs)]
//! This crate provides a low-level primitive for building Node.js-like event listeners.
//!
//! The 3 primitives are [`Bag`] that is a container for `Fn()` event handlers, [`BagOnce`] the same
//! for `FnOnce()` event handlers and [`HandlerId`] that will remove event handler from the bag on
//! drop.
//!
//! Trivial example:
//! ```rust
//! use event_listener_primitives::{Bag, HandlerId};
//!
//! fn main() {
//!     let bag = Bag::default();
//!
//!     let handler_id = bag.add(move || {
//!         println!("Hello")
//!     });
//!
//!     bag.call_simple();
//! }
//! ```
//!
//! Close to real-world usage example:
//! ```rust
//! use event_listener_primitives::{Bag, BagOnce, HandlerId};
//!
//! #[derive(Default)]
//! struct Handlers {
//!     action: Bag<Box<dyn Fn() + Send + Sync + 'static>>,
//!     closed: BagOnce<Box<dyn FnOnce() + Send + Sync + 'static>>,
//! }
//!
//! pub struct Container {
//!     handlers: Handlers,
//! }
//!
//! impl Drop for Container {
//!     fn drop(&mut self) {
//!         self.handlers.closed.call_simple();
//!     }
//! }
//!
//! impl Container {
//!     pub fn new() -> Self {
//!         let handlers = Handlers::default();
//!
//!         Self { handlers }
//!     }
//!
//!     pub fn do_action(&self) {
//!         // Do things...
//!
//!         self.handlers.action.call_simple();
//!     }
//!
//!     pub fn do_other_action(&self) {
//!         // Do things...
//!
//!         self.handlers.action.call(|callback| {
//!             callback();
//!         });
//!     }
//!
//!     pub fn on_action<F: Fn() + Send + Sync + 'static>(&self, callback: F) -> HandlerId {
//!         self.handlers.action.add(Box::new(callback))
//!     }
//!
//!     pub fn on_closed<F: FnOnce() + Send + Sync + 'static>(&self, callback: F) -> HandlerId {
//!         self.handlers.closed.add(Box::new(callback))
//!     }
//! }
//!
//! fn main() {
//!     let container = Container::new();
//!     let on_action_handler_id = container.on_action(|| {
//!         println!("On action");
//!     });
//!     container
//!         .on_closed(|| {
//!             println!("On container closed");
//!         })
//!         .detach();
//!     // This will trigger "action" callback just fine since its handler ID is not dropped yet
//!     container.do_action();
//!     drop(on_action_handler_id);
//!     // This will not trigger "action" callback since its handler ID was already dropped
//!     container.do_other_action();
//!     // This will trigger "closed" callback though since we've detached handler ID
//!     drop(container);
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

mod handler_id;
mod once;
mod regular;

pub use handler_id::HandlerId;
pub use once::BagOnce;
pub use regular::Bag;
