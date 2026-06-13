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
            // Wake all waiters so a queued writer cannot be stranded behind a
            // reader that wakes up only to block again on writer priority.
            self.cond.notify_all();
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
mod tests;
