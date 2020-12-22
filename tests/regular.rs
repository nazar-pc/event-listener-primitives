mod regular {
    use event_listener_primitives::Bag;
    use parking_lot::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn trivial() {
        let bag = Bag::default();
        let calls = Arc::new(AtomicUsize::new(0));

        let handler_id = {
            let calls = Arc::clone(&calls);
            bag.add(move || {
                calls.fetch_add(1, Ordering::SeqCst);
            })
        };

        bag.call_simple();

        assert_eq!(calls.load(Ordering::SeqCst), 1);

        drop(handler_id);
    }

    #[test]
    fn regular() {
        let bag = Bag::<Box<dyn Fn() + Send + Sync + 'static>>::default();
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
            })))
        }
        bag.call(|callback| {
            callback();
        });

        assert_eq!(calls.load(Ordering::SeqCst), 1);

        bag.call_simple();

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn deadlock_on_drop() {
        let bag = Bag::default();

        let (tx1, rx1) = std::sync::mpsc::sync_channel::<()>(0);
        let (tx2, rx2) = std::sync::mpsc::sync_channel::<()>(1);
        let handler_id = bag.add({
            let tx1 = Mutex::new(Some(tx1));
            let rx2 = Mutex::new(Some(rx2));

            move || {
                let _ = tx1.lock().take().unwrap().send(());
                let _ = rx2.lock().take().unwrap().recv();
            }
        });

        thread::spawn(move || {
            let _ = rx1.recv();
            drop(handler_id);
            let _ = tx2.send(());
        });

        bag.call_simple();
    }
}
