//! Concurrency primitives used by the engine.
//!
//! The main primitive is [`WriterPrefRwLock`]: a reader/writer lock with strict
//! writer priority. New readers are blocked while any writer is waiting, which
//! prevents writer starvation under a constant stream of read requests.

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::{Condvar, Mutex};

struct LockState {
    readers: usize,
    writers_waiting: usize,
    writer_active: bool,
}

/// A writer-preferring reader/writer lock.
///
/// Multiple readers may hold the lock simultaneously. Writers are granted the
/// lock exclusively, and once a writer is waiting no new readers are admitted
/// until all waiting writers have been served.
///
/// This lock is `Send` when the protected data is `Send`, and `Sync` when the
/// protected data is `Send + Sync`, matching the behaviour of `std::sync::RwLock`.
pub struct WriterPrefRwLock<T> {
    state: Mutex<LockState>,
    cond: Condvar,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for WriterPrefRwLock<T> {}
unsafe impl<T: Send + Sync> Sync for WriterPrefRwLock<T> {}

impl<T> WriterPrefRwLock<T> {
    /// Create a new writer-preferring lock protecting `data`.
    pub fn new(data: T) -> Self {
        Self {
            state: Mutex::new(LockState {
                readers: 0,
                writers_waiting: 0,
                writer_active: false,
            }),
            cond: Condvar::new(),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquire a read lock. Blocks if a writer currently holds the lock or if
    /// any writer is already waiting.
    pub fn read(&self) -> ReadGuard<'_, T> {
        let mut state = self.state.lock().expect("writer-pref lock poisoned");
        while state.writer_active || state.writers_waiting > 0 {
            state = self.cond.wait(state).expect("writer-pref condvar poisoned");
        }
        state.readers += 1;
        ReadGuard { lock: self }
    }

    /// Acquire a write lock. Blocks until all current readers and writers have
    /// released the lock.
    pub fn write(&self) -> WriteGuard<'_, T> {
        let mut state = self.state.lock().expect("writer-pref lock poisoned");
        state.writers_waiting += 1;
        while state.readers > 0 || state.writer_active {
            state = self.cond.wait(state).expect("writer-pref condvar poisoned");
        }
        state.writers_waiting -= 1;
        state.writer_active = true;
        WriteGuard { lock: self }
    }

    fn release_read(&self) {
        let mut state = self.state.lock().expect("writer-pref lock poisoned");
        state.readers -= 1;
        if state.readers == 0 {
            // Wake one waiter. If a writer is queued it will acquire the lock;
            // otherwise a reader may acquire it (but only if no writer is waiting).
            self.cond.notify_one();
        }
    }

    fn release_write(&self) {
        let mut state = self.state.lock().expect("writer-pref lock poisoned");
        state.writer_active = false;
        self.cond.notify_all();
    }

    /// Number of threads currently holding a read lock.
    pub fn active_readers(&self) -> usize {
        self.state
            .lock()
            .expect("writer-pref lock poisoned")
            .readers
    }

    /// Number of writers currently waiting for the lock.
    pub fn waiting_writers(&self) -> usize {
        self.state
            .lock()
            .expect("writer-pref lock poisoned")
            .writers_waiting
    }

    /// Whether a writer currently holds the lock.
    pub fn writer_active(&self) -> bool {
        self.state
            .lock()
            .expect("writer-pref lock poisoned")
            .writer_active
    }
}

/// RAII guard for a read lock.
pub struct ReadGuard<'a, T> {
    lock: &'a WriterPrefRwLock<T>,
}

impl<T> Deref for ReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // Safety: the lock state guarantees shared access is valid.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> Drop for ReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.release_read();
    }
}

/// RAII guard for a write lock.
pub struct WriteGuard<'a, T> {
    lock: &'a WriterPrefRwLock<T>,
}

impl<T> Deref for WriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // Safety: the lock state guarantees exclusive access is valid.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // Safety: the lock state guarantees exclusive access is valid.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for WriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.release_write();
    }
}

/// A simple counting semaphore used to preserve the actor-style backpressure
/// limit on the number of concurrent database operations.
pub struct CapacitySemaphore {
    state: Mutex<SemaphoreState>,
    cond: Condvar,
    capacity: usize,
}

struct SemaphoreState {
    available: usize,
    waiting: usize,
}

/// RAII permit for [`CapacitySemaphore`].
pub struct SemaphorePermit<'a> {
    semaphore: &'a CapacitySemaphore,
}

impl CapacitySemaphore {
    /// Create a semaphore with the given total capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            state: Mutex::new(SemaphoreState {
                available: capacity,
                waiting: 0,
            }),
            cond: Condvar::new(),
            capacity,
        }
    }

    /// Acquire a permit, blocking until one is available.
    pub fn acquire(&self) -> SemaphorePermit<'_> {
        let mut state = self.state.lock().expect("semaphore lock poisoned");
        state.waiting += 1;
        while state.available == 0 {
            state = self.cond.wait(state).expect("semaphore condvar poisoned");
        }
        state.waiting -= 1;
        state.available -= 1;
        SemaphorePermit { semaphore: self }
    }

    /// Try to acquire a permit without blocking.
    pub fn try_acquire(&self) -> Option<SemaphorePermit<'_>> {
        let mut state = self.state.lock().expect("semaphore lock poisoned");
        if state.available == 0 {
            return None;
        }
        state.available -= 1;
        Some(SemaphorePermit { semaphore: self })
    }

    /// Current number of available permits.
    pub fn available_permits(&self) -> usize {
        self.state
            .lock()
            .expect("semaphore lock poisoned")
            .available
    }

    /// Total capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    fn release(&self) {
        let mut state = self.state.lock().expect("semaphore lock poisoned");
        state.available += 1;
        if state.waiting > 0 {
            self.cond.notify_one();
        }
    }
}

impl Drop for SemaphorePermit<'_> {
    fn drop(&mut self) {
        self.semaphore.release();
    }
}

#[cfg(test)]
mod tests {
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
}
