## Event listener primitives

[![Build Status](https://img.shields.io/travis/com/nazar-pc/event-listener-primitives/master?style=flat-square)](https://travis-ci.com/nazar-pc/event-listener-primitives)
[![Crates.io](https://img.shields.io/crates/v/event-listener-primitives?style=flat-square)](https://crates.io/crates/event-listener-primitives)
[![Docs](https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square)](https://docs.rs/event-listener-primitives)
[![License](https://img.shields.io/github/license/nazar-pc/event-listener-primitives?style=flat-square)](https://github.com/nazar-pc/event-listener-primitives)

This crate provides a low-level primitive for building Node.js-like event listeners.

The 3 primitives are `Bag` that is a container for `Fn()` event handlers, `BagOnce` the same for `FnOnce()` event handlers and `HandlerId` that will remove event handler from the bag on drop.

Trivial example:
```rust
use event_listener_primitives::{Bag, HandlerId};
use std::sync::Arc;

fn main() {
    let bag = Bag::default();

    let handler_id = bag.add(Arc::new(|| {
        println!("Hello");
    }));

    bag.call_simple();
}
```

Close to real-world usage example:

```rust
use event_listener_primitives::{Bag, BagOnce, HandlerId};
use std::sync::Arc;

#[derive(Default)]
struct Handlers {
    action: Bag<Arc<dyn Fn() + Send + Sync + 'static>>,
    closed: BagOnce<Box<dyn FnOnce() + Send + 'static>>,
}

pub struct Container {
    handlers: Handlers,
}

impl Drop for Container {
    fn drop(&mut self) {
        self.handlers.closed.call_simple();
    }
}

impl Container {
    pub fn new() -> Self {
        let handlers = Handlers::default();

        Self { handlers }
    }

    pub fn do_action(&self) {
        // Do things...

        self.handlers.action.call_simple();
    }

    pub fn do_other_action(&self) {
        // Do things...

        self.handlers.action.call(|callback| {
            callback();
        });
    }

    pub fn on_action<F: Fn() + Send + Sync + 'static>(&self, callback: F) -> HandlerId {
        self.handlers.action.add(Arc::new(callback))
    }

    pub fn on_closed<F: FnOnce() + Send + 'static>(&self, callback: F) -> HandlerId {
        self.handlers.closed.add(Box::new(callback))
    }
}

fn main() {
    let container = Container::new();
    let on_action_handler_id = container.on_action(|| {
        println!("On action");
    });
    container
        .on_closed(|| {
            println!("On container closed");
        })
        .detach();
    // This will trigger "action" callback just fine since its handler ID is not dropped yet
    container.do_action();
    drop(on_action_handler_id);
    // This will not trigger "action" callback since its handler ID was already dropped
    container.do_other_action();
    // This will trigger "closed" callback though since we've detached handler ID
    drop(container);

    println!("Done");
}
```

The output will be:
```text
On action
On closed
Done
```

## Contribution
Feel free to create issues and send pull requests, they are highly appreciated!

## License
Zero-Clause BSD

<https://opensource.org/licenses/0BSD>

<https://tldrlegal.com/license/bsd-0-clause-license>
