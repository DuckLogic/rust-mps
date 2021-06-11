//! Supported pools
use mps_sys::{mps_pool_t};

use crate::arena::Arena;

pub mod mark_sweep;

/// A pool of memory managed by the Memory Pool System
///
/// Pools are responsible for requesting memory from the arena.
/// The MPS is designed for pools of different classes to co-exist in the same arena,
/// so that objects requiring different memory management policies
/// can be segregated into pools of suitable classes.
///
/// Some pools are automatically managed (garbage collected),
/// while others are manually managed (malloc/free)
///
/// See MPS documentation on how to best choose a pool class.
pub unsafe trait Pool<'arena> {
    /// Get the raw type of the pool
    unsafe fn as_raw(&self) -> mps_pool_t;
    /// Get the underlying MPS [Arena]
    fn arena(&self) -> &'arena Arena;
    /// Return the total memory allocated
    /// from the arena and managed by the pool.
    #[inline]
    fn total_size(&self) -> usize {
        unsafe {
            mps_sys::mps_pool_total_size(self.as_raw())
        }
    }
    /// Return the free memory: memory managed by the pool
    /// but not in use by the client program.
    #[inline]
    fn free_size(&self) -> usize {
        unsafe {
            mps_sys::mps_pool_free_size(self.as_raw())
        }
    }
    /// Return if this pool automatically manages memory
    fn is_automatic(&self) -> bool;
    /// Return if this pool manually manages memory
    #[inline]
    fn is_manual(&self) -> bool {
        !self.is_automatic()
    }
}

/// A pool that supports automatic garbage collection
pub unsafe trait AutomaticPool<'arena>: Pool<'arena> {}
