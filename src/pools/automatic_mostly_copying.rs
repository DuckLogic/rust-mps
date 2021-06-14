//! Support for the [Automatic Mostly Copying](https://www.ravenbrook.com/project/mps/master/manual/html/pool/amc.html) pool
//!
//! It is the most mature pool class in the MPS, and is the one primarily intended for production use.

use crate::arena::Arena;
use mps_sys::{mps_pool_t, mps_kw_arg, mps_pool_create_k, mps_pool_destroy};
use std::mem::ManuallyDrop;
use crate::format::ObjectFormat;
use crate::pools::{AutomaticPool, Pool};
use arrayvec::ArrayVec;
use crate::MpsError;

/// A builder for [AMC pools](AutoMostlyCopyingPool)
pub struct AutoMostlyCopyingBuilder<'a> {
    arena: &'a Arena,
    allow_interior: Option<bool>,
    extend_by: Option<usize>
}
impl<'a> AutoMostlyCopyingBuilder<'a> {
    /// Specify whether ambiguous interior pointers to blocks
    /// in the pool keep objects alive.
    ///
    /// If this is false, the only "client pointers" keep objects alive.
    #[inline]
    pub fn allow_interior(&mut self, b: bool) -> &mut Self {
        self.allow_interior = Some(b);
        self
    }
    /// Specify the minimum size of the memory segments that the pool requests
    /// from the underlying arena.
    ///
    /// Larger segments reduce per-segment overhead, but increase fragmentation
    /// and retention.
    #[inline]
    pub fn extend_by(&mut self, size: usize) -> &mut Self {
        self.extend_by = Some(size);
        self
    }
    /// Finish building the pool, using the specified [object format](ObjectFormat)
    #[inline]
    pub fn build(&self, format: ObjectFormat<'a>) -> Result<AutoMostlyCopyingPool<'a>, MpsError> {
        unsafe {
            let mut args = ArrayVec::<_, 4>::new();
            args.push(mps_kw_arg!(FORMAT => format.as_raw()));
            args.push(::mps_sys::mps_args_end());
            let mut pool = std::ptr::null_mut();
            let format = ManuallyDrop::new(format);
            handle_mps_res!(mps_pool_create_k(
                &mut pool, self.arena.as_raw(),
                ::mps_sys::mps_class_amc(),
                args.as_mut_ptr()
            ))?;
            assert!(!pool.is_null());
            Ok(AutoMostlyCopyingPool {
                raw: pool, format,
                arena: self.arena
            })
        }
    }
}

/// The [automatic, mostly copying](https://www.ravenbrook.com/project/mps/master/manual/html/pool/amc.html#amc-automatic-mostly-copying) [Pool]
pub struct AutoMostlyCopyingPool<'a> {
    raw: mps_pool_t,
    // Must drop after pool
    format: ManuallyDrop<ObjectFormat<'a>>,
    arena: &'a Arena
}
impl<'a> AutoMostlyCopyingPool<'a> {
    /// Begin to build a new automatic, mostly copying pool
    ///
    /// See [the docs](https://www.ravenbrook.com/project/mps/master/manual/html/pool/amc.html#c.mps_class_amc)
    /// for more details on the available options.
    #[inline]
    pub fn builder(arena: &'a Arena) -> AutoMostlyCopyingBuilder<'a> {
        AutoMostlyCopyingBuilder {
            arena,
            allow_interior: None,
            extend_by: None
        }
    }
}
unsafe impl<'a> Pool<'a> for AutoMostlyCopyingPool<'a> {
    #[inline]
    unsafe fn as_raw(&self) -> mps_pool_t {
        self.raw
    }
    #[inline]
    fn arena(&self) -> &'a Arena {
        self.arena
    }
    #[inline]
    fn is_automatic(&self) -> bool {
        true
    }
}
unsafe impl<'a> AutomaticPool<'a> for AutoMostlyCopyingPool<'a> {}
unsafe impl<'a> Send for AutoMostlyCopyingPool<'a> {}
/// This is thread safe
///
/// <https://www.ravenbrook.com/project/mps/master/manual/html/design/thread-safety.html>
unsafe impl<'a> Sync for AutoMostlyCopyingPool<'a> {}
impl<'a> Drop for AutoMostlyCopyingPool<'a> {
    fn drop(&mut self) {
        // NOTE: Drop pool *before* format
        unsafe {
            mps_pool_destroy(self.raw);
            ManuallyDrop::drop(&mut self.format);
        }
    }
}