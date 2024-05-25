use std::sync::{
    atomic::{AtomicI32, Ordering},
    Arc,
};

/// App context that includes the current counter value.
#[derive(Clone, Default)]
pub struct Ctx {
    counter: Arc<AtomicI32>,
}

impl Ctx {
    /// Increment the counter.
    pub fn increment(&self) {
        (*self.counter).fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement the counter.
    pub fn decrement(&self) {
        (*self.counter).fetch_sub(1, Ordering::Relaxed);
    }

    /// Add some value to the counter.
    pub fn add(&self, n: i32) {
        (*self.counter).fetch_add(n, Ordering::Relaxed);
    }

    /// Get the current counter value.
    pub fn get(&self) -> i32 {
        (*self.counter).load(Ordering::Relaxed)
    }
}
