//! Support for the automatic mark/sweep pool
use arrayvec::ArrayVec;
use mps_sys::*;
use crate::format::ObjectFormat;
use crate::arena::Arena;
use std::mem::{ManuallyDrop, MaybeUninit};
use crate::MpsError;

use super::{Pool, AutomaticPool};
use std::ffi::c_void;

/// Debug options for a [AutoMarkSweep] collector
///
/// See [debug docs](https://www.ravenbrook.com/project/mps/master/manual/html/topic/debugging.html#debugging-pools) for more info.
pub struct DebugOptions {
    /// The template to write a fencepost with.
    ///
    /// These are written before and after each allocated block.
    pub fence_template: Option<&'static [u8]>,
    /// The template to overwrite free code with.
    pub free_template: Option<&'static [u8]>
}
impl Default for DebugOptions {
    fn default() -> Self {
        DebugOptions {
            fence_template: Some(b"FENCE \xDE\xAD\xBE\xEF"),
            free_template: Some(b"FREE \xCA\xFE\xBA\xBE")
        }
    }
}

/// Builds a [AutoMarkSweep] collector
pub struct AutoMarkSweepBuilder<'a> {
    arena: &'a Arena,
    debug: Option<DebugOptions>,
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
    /// Switch to using the [debug pool](https://www.ravenbrook.com/project/mps/master/manual/html/topic/debugging.html#debugging-pools),
    /// configuring it with the specified options
    #[inline]
    #[deprecated(note = "Seems buggy last time I tried it")]
    pub fn debug(&mut self, opts: Option<DebugOptions>) -> &mut Self {
        self.debug = opts;
        self
    }
    /// Build the pool, using the specified
    /// object format to scan objects.
    pub fn build(&mut self, format: ObjectFormat<'a>) -> Result<AutoMarkSweep<'a>, MpsError> {
        unsafe {
            let raw_class = match self.debug {
                Some(_) => mps_sys::mps_class_ams_debug(),
                None => mps_sys::mps_class_ams(),
            };
            let mut args = ArrayVec::<_, 4>::new();
            args.push(mps_kw_arg!(FORMAT => format.as_raw()));
            if let Some(ambiguous) = self.allow_ambiguous {
                args.push(mps_kw_arg!(AMS_SUPPORT_AMBIGUOUS => ambiguous));
            }
            let mut debug_options: MaybeUninit<mps_pool_debug_option_s> = MaybeUninit::uninit();
            if let Some(ref debug) = self.debug {
                debug_options.as_mut_ptr().write(mps_pool_debug_option_s {
                    free_template: debug.free_template
                        .map(|s| s.as_ptr() as *const c_void)
                        .unwrap_or(std::ptr::null()),
                    free_size: debug.free_template.map_or(0, |s| s.len()),
                    fence_template: debug.fence_template
                        .map(|s| s.as_ptr() as *const c_void)
                        .unwrap_or(std::ptr::null()),
                    fence_size: debug.fence_template.map_or(0, |s| s.len())
                });
                args.push(mps_kw_arg!(POOL_DEBUG_OPTIONS => debug_options.as_mut_ptr()))
            }
            args.push(mps_sys::mps_args_end());
            let mut pool = std::ptr::null_mut();
            let format = ManuallyDrop::new(format);
            handle_mps_res!(mps_pool_create_k(
                &mut pool, self.arena.as_raw(),
                raw_class,
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
            debug: None,
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
        // NOTE: Drop pool *before* format
        unsafe {
            mps_pool_destroy(self.raw);
            ManuallyDrop::drop(&mut self.format);
        }
    }
}

