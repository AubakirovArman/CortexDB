use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use super::WriterPrefRwLock;

#[test]
fn multiple_readers_hold_lock_concurrently() {
    let lock = Arc::new(WriterPrefRwLock::new(AtomicUsize::new(0)));
    let mut handles = Vec::new();
    for _ in 0..8 {
        let lock = Arc::clone(&lock);
        handles.push(thread::spawn(move || {
            let guard = lock.read();
            guard.fetch_add(1, Ordering::SeqCst);
            // Hold briefly to increase chance of overlap.
            thread::sleep(Duration::from_millis(5));
            assert!(guard.load(Ordering::SeqCst) >= 1);
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    assert_eq!(lock.read().load(Ordering::SeqCst), 8);
}

#[test]
fn writer_blocks_until_readers_leave() {
    let lock = Arc::new(WriterPrefRwLock::new(0usize));
    let read_guard = lock.read();
    let wrote = Arc::new(AtomicUsize::new(0));
    let wrote_clone = Arc::clone(&wrote);
    let lock_clone = Arc::clone(&lock);
    let handle = thread::spawn(move || {
        let mut guard = lock_clone.write();
        *guard += 1;
        wrote_clone.store(1, Ordering::SeqCst);
    });
    thread::sleep(Duration::from_millis(20));
    assert_eq!(Arc::clone(&wrote).load(Ordering::SeqCst), 0);
    drop(read_guard);
    handle.join().unwrap();
    assert_eq!(wrote.load(Ordering::SeqCst), 1);
    assert_eq!(*lock.read(), 1);
}

#[test]
fn waiting_writer_blocks_new_readers() {
    let lock = Arc::new(WriterPrefRwLock::new(0usize));
    let read_guard = lock.read();
    let lock_clone = Arc::clone(&lock);
    let writer_started = Arc::new(AtomicUsize::new(0));
    let writer_started_clone = Arc::clone(&writer_started);
    let writer_handle = thread::spawn(move || {
        writer_started_clone.store(1, Ordering::SeqCst);
        let mut guard = lock_clone.write();
        *guard = 42;
    });
    // Wait until the writer has actually entered the wait queue.
    while writer_started.load(Ordering::SeqCst) == 0 {
        thread::yield_now();
    }
    thread::sleep(Duration::from_millis(10));

    // A new reader should not be able to acquire while a writer is waiting.
    let lock_clone2 = Arc::clone(&lock);
    let reader_value = Arc::new(AtomicUsize::new(0));
    let reader_value_clone = Arc::clone(&reader_value);
    let reader_handle = thread::spawn(move || {
        let guard = lock_clone2.read();
        reader_value_clone.store(*guard, Ordering::SeqCst);
    });
    thread::sleep(Duration::from_millis(20));
    assert_eq!(reader_value.load(Ordering::SeqCst), 0);

    drop(read_guard);
    writer_handle.join().unwrap();
    reader_handle.join().unwrap();
    assert_eq!(reader_value.load(Ordering::SeqCst), 42);
}

#[test]
fn writer_does_not_starve_under_reader_spam() {
    let lock = Arc::new(WriterPrefRwLock::new(0usize));
    let mut writer_waits = Vec::new();
    for _ in 0..4 {
        let lock = Arc::clone(&lock);
        writer_waits.push(thread::spawn(move || {
            let start = Instant::now();
            let mut guard = lock.write();
            *guard += 1;
            start.elapsed()
        }));
    }
    // Spawn a stream of readers that would starve writers without priority.
    let reader_stop = Arc::new(AtomicUsize::new(0));
    let mut reader_handles = Vec::new();
    for _ in 0..4 {
        let lock = Arc::clone(&lock);
        let stop = Arc::clone(&reader_stop);
        reader_handles.push(thread::spawn(move || {
            while stop.load(Ordering::SeqCst) == 0 {
                let _ = lock.read();
                thread::yield_now();
            }
        }));
    }
    let max_wait = writer_waits
        .into_iter()
        .map(|h| h.join().unwrap())
        .max()
        .unwrap();
    reader_stop.store(1, Ordering::SeqCst);
    for h in reader_handles {
        h.join().unwrap();
    }
    // Without writer priority this would typically take hundreds of ms.
    assert!(
        max_wait < Duration::from_millis(500),
        "writer starved: waited {max_wait:?}"
    );
    assert_eq!(*lock.read(), 4);
}
