#![allow(dead_code)]
use std::sync::atomic::{self, Ordering};
// use std::ops::{Sub, Add, Mul, Div};
pub trait AtomicOps {
    type Item: Copy;
    fn new(v: Self::Item) -> Self;
    /// Loads a value from the atomic integer with relaxed ordering.
    fn get(&self) -> Self::Item;
    /// Stores a value into the atomic integer with relaxed ordering.
    fn set(&self, v: Self::Item);
}
#[derive(Debug)]
/// Simple 32-bit floating point wrapper over `AtomicU32` with relaxed ordering.
pub struct AtomicF32(atomic::AtomicU32);
impl AtomicOps for AtomicF32 {
    type Item = f32;
    /// Create a new atomic 32-bit float with initial value `v`.
    fn new(v: f32) -> Self {
        AtomicF32(atomic::AtomicU32::new(v.to_bits()))
    }
    /// Loads a value from the atomic float with relaxed ordering.
    #[inline]
    fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
    /// Stores a value into the atomic float with relaxed ordering.
    #[inline]
    fn set(&self, v: f32) {
        self.0.store(v.to_bits(), Ordering::Relaxed)
    }
}
/// Simple wrapper over `AtomicBool` with relaxed ordering.
pub struct AtomicBool(pub atomic::AtomicBool);
#[allow(dead_code)]
impl AtomicOps for AtomicBool {
    type Item = bool;
    /// Create a new atomic 8-bit integer with initial value `v`.
    fn new(v: bool) -> AtomicBool {
        AtomicBool(atomic::AtomicBool::new(v))
    }
    #[inline]
    /// Loads a value from the atomic integer with relaxed ordering.
    fn get(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
    #[inline]
    /// Stores a value into the atomic integer with relaxed ordering.
    fn set(&self, v: bool) {
        self.0.store(v, Ordering::Release)
    }
}
impl AtomicBool {
    #[inline]
    pub fn check_reset(&self) -> bool {
        self.0
            .compare_exchange(
                true,
                false,
                std::sync::atomic::Ordering::Acquire,
                std::sync::atomic::Ordering::Relaxed,
            )
            .is_ok()
    }
    #[inline]
    pub fn set_release(&self, v: bool) {
        self.0.store(v, Ordering::Release)
    }
}
