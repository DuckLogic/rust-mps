//! Support for the automatic mark/sweep pool
use mps_sys::*;
use crate::format::ObjectFormat;
use crate::arena::Arena;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use crate::MpsError;

/// Automatically managed mark/sweep garbage collection.
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
    arena: PhantomData<&'a Arena>
}
impl<'a> AutoMarkSweep<'a> {
    /// Create a new mark/sweep pool with the specified object format
    pub fn new(arena: &'a Arena, format: ObjectFormat<'a>) -> Result<Self, MpsError> {
        assert!(format.managed());
        unsafe {
            let mut args = mps_kw_args!(
                FORMAT => format.as_raw()
            );
            let mut pool = std::ptr::null_mut();
            handle_mps_res!(mps_pool_create_k(
                &mut pool, arena.as_raw(), mps_class_ams(),
                args.as_mut_ptr()
            ))?;
            assert!(!pool.is_null());
            Ok(AutoMarkSweep {
                raw: pool, format: ManuallyDrop::new(format),
                arena: PhantomData
            })
        }
    }
}
impl<'a> Drop for AutoMarkSweep<'a> {
    fn drop(&mut self) {
        // NOTE: pool before format
        unsafe {
            ManuallyDrop::drop(&mut self.format);
            mps_pool_destroy(self.raw);
        }
    }
}

