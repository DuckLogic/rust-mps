//! Support for the automatic mark/sweep pool
use arrayvec::ArrayVec;
use mps_sys::*;
use crate::format::ObjectFormat;
use crate::arena::Arena;
use std::mem::ManuallyDrop;
use crate::MpsError;

use super::{Pool, AutomaticPool};

/// Builds a [AutoMarkSweep] collector
pub struct AutoMarkSweepBuilder<'a> {
    raw_class: mps_pool_class_t,
    arena: &'a Arena,
    allow_ambiguous: Option<bool>,
}
impl<'a> AutoMarkSweepBuilder<'a> {
    /// Specify whether references to blocks in the pool
    /// may be ambiguous.
    ///
    /// For Automatically Mark Sweep,
    /// the default is true.
    #[inline]
    pub fn allow_ambiguous(&mut self, b: bool) -> &mut Self {
        self.allow_ambiguous = Some(b);
        self
    }
    /// Build the pool, using the specified
    /// object format to scan objects.
    pub fn build(&mut self, format: ObjectFormat<'a>) -> Result<AutoMarkSweep<'a>, MpsError> {
        unsafe {
            let mut args = ArrayVec::<_, 3>::new();
            args.push(mps_kw_arg!(FORMAT => format.as_raw()));
            if let Some(ambiguous) = self.allow_ambiguous {
                args.push(mps_kw_arg!(AMS_SUPPORT_AMBIGUOUS => ambiguous));
            }
            args.push(mps_sys::mps_args_end());
            let mut pool = std::ptr::null_mut();
            let format = ManuallyDrop::new(format);
            handle_mps_res!(mps_pool_create_k(
                &mut pool, self.arena.as_raw(),
                self.raw_class,
                args.as_mut_ptr()
            ))?;
            assert!(!pool.is_null());
            Ok(AutoMarkSweep {
                raw: pool, format,
                arena: self.arena
            })
        }
    }
}


/// An [Automatic mark/sweep](https://www.ravenbrook.com/project/mps/master/manual/html/pool/ams.html#pool-ams)
/// (or "AMS") [Pool](crate::pool::Pool)
///
/// This doesn't move memory, so it's useful as a first step
/// using the MPS.
///
/// According to the docs, it's not "suitable for production use"
/// but is still useful for debugging.
pub struct AutoMarkSweep<'a> {
    raw: mps_pool_t,
    // Must drop after pool
    format: ManuallyDrop<ObjectFormat<'a>>,
    arena: &'a Arena
}
impl<'a> AutoMarkSweep<'a> {
    /// Begin to build a new automatic mark sweep pool
    ///
    /// See [the docs](https://www.ravenbrook.com/project/mps/master/manual/html/pool/ams.html#c.mps_class_ams)
    /// for more details on the available options.
    #[inline]
    pub fn builder(arena: &'a Arena) -> AutoMarkSweepBuilder<'a> {
        AutoMarkSweepBuilder {
            raw_class: unsafe { mps_sys::mps_class_ams() },
            arena,
            allow_ambiguous: None
        }
    }
}
unsafe impl<'a> Pool<'a> for AutoMarkSweep<'a> {
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
unsafe impl<'a> AutomaticPool<'a> for AutoMarkSweep<'a> {}
unsafe impl<'a> Send for AutoMarkSweep<'a> {}
/// This is thread safe
///
/// <https://www.ravenbrook.com/project/mps/master/manual/html/design/thread-safety.html>
unsafe impl<'a> Sync for AutoMarkSweep<'a> {}
impl<'a> Drop for AutoMarkSweep<'a> {
    fn drop(&mut self) {
        // NOTE: Drop pool after format
        unsafe {
            ManuallyDrop::drop(&mut self.format);
            mps_pool_destroy(self.raw);
        }
    }
}

