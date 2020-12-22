mod once {
    use event_listener_primitives::BagOnce;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn once() {
        let bag = BagOnce::default();
        let calls = Arc::new(AtomicUsize::new(0));

        {
            let calls = Arc::clone(&calls);
            bag.add(move || {
                calls.fetch_add(1, Ordering::SeqCst);
            })
            .detach();
        }
        {
            let calls = Arc::clone(&calls);
            drop(bag.add(move || {
                calls.fetch_add(1, Ordering::SeqCst);
            }));
        }
        bag.call(|callback| {
            callback();
        });

        assert_eq!(calls.load(Ordering::SeqCst), 1);

        {
            let calls = Arc::clone(&calls);
            bag.add(move || {
                calls.fetch_add(1, Ordering::SeqCst);
            })
            .detach();
        }
        bag.call_simple();

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn deadlock_on_drop() {
        let bag = BagOnce::default();

        let (tx1, rx1) = std::sync::mpsc::sync_channel::<()>(0);
        let (tx2, rx2) = std::sync::mpsc::sync_channel::<()>(1);
        let handler_id = bag.add(move || {
            let _ = tx1.send(());
            let _ = rx2.recv();
        });

        thread::spawn(move || {
            let _ = rx1.recv();
            drop(handler_id);
            let _ = tx2.send(());
        });

        bag.call_simple();
    }
}
