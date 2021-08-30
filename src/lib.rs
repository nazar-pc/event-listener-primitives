#![warn(rust_2018_idioms, missing_debug_implementations, missing_docs)]
#![doc = include_str!("../readme.md")]

mod handler_id;
mod once;
mod regular;

pub use handler_id::HandlerId;
pub use once::BagOnce;
pub use regular::Bag;
