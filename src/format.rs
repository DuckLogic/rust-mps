//! Communicating object formats to the MPS
use std::os::raw::c_void;

use mps_sys::*;
use std::marker::PhantomData;
use crate::arena::Arena;
use crate::MpsError;
use arrayvec::ArrayVec;

/// An object format communicates the object's layout to the MPS.
///
/// Object formats include information on alignment and several
/// format methods.
/// This includes methods for tracing, "skipping" (determining size),
/// relocating ("forwarding")
pub struct ObjectFormat<'a> {
    raw: mps_fmt_t,
    managed: bool,
    _arena: PhantomData<&'a Arena>
}
impl<'a> ObjectFormat<'a> {
    #[inline]
    pub(crate) fn as_raw(&self) -> mps_fmt_t {
        self.raw
    }
    /// Create a new object format for use with managed
    /// (garbage collected) pools
    ///
    /// The format methods are taken from associated methods
    /// on `<M as RawFormatMethods>`
    ///
    /// The minimum alignment of an object is optionally given by `object_align`
    pub fn managed_with<M, T>(
        arena: &'a Arena,
        object_align: Option<usize>
    ) -> Result<ObjectFormat, MpsError>
        where M: RawFormatMethods<T> {
        let mut args: ArrayVec<[mps_arg_s; 8]> = ArrayVec::new();
        unsafe {
            if let Some(align) = object_align {
                assert!(align > 0 && align.is_power_of_two());
                args.push(mps_kw_arg!(FMT_ALIGN => align));
            }
            // TODO: HEADER_SIZE?
            args.extend(mps_kw_args!(
                FMT_SCAN => M::scan as unsafe extern "C" fn(_, _, _) -> _,
                FMT_SKIP => M::skip as unsafe extern "C" fn(_) -> _,
                FMT_FWD => M::forward as unsafe extern "C" fn(_, _) -> _,
                FMT_ISFWD => M::is_forwarded as unsafe extern "C" fn(_) -> _,
                FMT_PAD => M::pad as unsafe extern "C" fn(_, _) -> _,
                FMT_CLASS => M::class_ptr as unsafe extern "C" fn(_) -> _
            ));
            let mut fmt = std::ptr::null_mut();
            handle_mps_res!(mps_fmt_create_k(&mut fmt, arena.as_raw(), args.as_mut_ptr()))?;
            Ok(ObjectFormat { raw: fmt, managed: true, _arena: PhantomData })
        }
    }
    /// Whether the created object format was 'managed'
    ///
    /// Managed object formats have a set of [RawFormatMethods]
    #[inline]
    pub fn managed(&self) -> bool {
        self.managed
    }
}
unsafe impl Send for ObjectFormat<'_> {}
unsafe impl Sync for ObjectFormat<'_> {}
impl Drop for ObjectFormat<'_> {
    fn drop(&mut self) {
        /*
         * NOTE: Pool must die first
         * This is guarenteed on their end
         * since the wrapper owns a reference to us
         */
        unsafe { mps_fmt_destroy(self.raw) }
    }
}

/// MPS object format methods, for use with managed objects
///
/// ## Safety
/// 1. MPS guarantees that format methods have exclusive access
///    to objects for the duration of the call. This may involve pausing user threads.
///    Format methods may not lock or perform inter-thread communication
/// 2. Format methods may be called *from a signal handler*. On POSIX systems this
///    means format methods must be signal-safe. For example.
///    1. MPS places read barrier on some memory
///    2. client attempts to read from this block
///    3. SEGFAULT
///    4. MPS signal handler is called
///    5. MPS ensures the block is consistent
///    6. MPS calls some format methods
/// 3. Format methods must be re-entrant
/// 4. Format methods must never use more than 64 words of stack space
/// 5. Format methods may **never**.
///    1. Call library code
///    2. Perform a non-local exit (panic/exception/longjmp)
///    3. Call any MPS functions other than the special fixup/relocation functions
/// 6. However, given the above constraints are followed, format methods are free to:
///    1. Access memory inside the object/block they've been asked to examine
///    2. Access MPS memory that is in pools that doen't protect memory (unmanaged pools)
///    3. Access memory not managed by the MPS
pub unsafe trait RawFormatMethods<T: ?Sized> {
    /// Give an address related to the class of the object,
    /// or a null pointer if none is available.
    ///
    /// Padding and forwarding objects should return null
    unsafe extern "C" fn class_ptr(obj: *mut T) -> *mut c_void;
    /// The MPS calls the forward method for an object format when
    /// it has relocated an object belonging to that format.
    ///
    /// The forward method must replace the object at old
    /// with a forwarding marker that points to the address ‘new’.
    /// The forwarding marker must:
    /// 1. Be compatible with all the other methods in this format
    /// 2. Be the same size as the original object. In other words,
    ///    the "skip"/object size method must return the same result as the original.
    unsafe extern "C" fn forward(old: *mut T, new: *mut T);
    /// If the specified object is a forwarding object,
    /// return its new location.
    ///
    /// Otherwise return null.
    unsafe extern "C" fn is_forwarded(old: *mut T);
    /// Create a padding object, to fill in otherwise unused space.
    ///
    /// This method must create a padding object of the specified size
    /// at the given target address. Any alignment (compatible with the format)
    /// is acceptable, but the resulting padding
    /// object must be compatible with all other format methods.
    ///
    /// The MPS typically uses this to pack objects into fixed sized units
    /// (such as OS pages).
    unsafe extern "C" fn pad(addr: *mut T, size: usize);
    /// Called when the MPS needs to scan (and relocate) objects in a block of memory
    /// that belong to this format.
    ///
    /// Base points to the first formatted object in the block of memory (inclusive),
    /// while limit is the location just beyond the end of the block (exclusive).
    ///
    /// The scan state must be passed to `ScanState::fix_with` before fixing references.
    ///
    /// If the object format is capable of creating forwarding objects or padding objects,
    /// the scan method must be able to scan these objects.
    /// The scan method must *never fixup forwarding objects*.
    unsafe extern "C" fn scan(state: ScanState, base: *mut T, limit: *mut T) -> mps_res_t;
    /// Return the address of the next object (implicitly computing its size).
    ///
    /// If this format has no headers, this is the address just past the end of the object.
    ///
    /// If the format does have in-band headers, they should be excluded.
    ///
    /// If this format creates forwarding or padding objects,
    /// this method must be able to handle them.
    ///
    /// This method must be infallible.
    unsafe extern "C" fn skip(addr: *mut T) -> *mut T;
}
/// The initial scan state passed to an object format
#[repr(transparent)]
pub struct ScanState {
    #[doc(hidden)]
    raw: mps_ss_t
}
impl ScanState {
    /// Begin to setup the fix state to scan a set of objects.
    ///
    /// Within this closure, the `ScanFixState` is in a special state
    /// and shouldn't be passed to external functions.
    #[inline(always)] // MPS implements all of this as macros. Does it have to be inline?
    pub unsafe fn fix_with<F>(&mut self, func: F) -> mps_res_t
        where F: FnOnce(&mut ScanFixState) -> Result<(), mps_res_t> {
        // See: MPS_SCAN_BEGIN
        let mut state = ScanFixState {
            state: self.raw,
            zs: (*self.raw)._zs,
            w: (*self.raw)._w,
            ufs: (*self.raw)._ufs,
        };
        match func(&mut state) {
            Ok(()) => {},
            Err(code) => return code
        }
        // See: MPS_SCAN_END
        (*self.raw)._ufs = state.ufs;
        0 // MPS_RES_OK
    }
}

/// A scan state in the necessary state
/// to fixup references
///
/// This is a very special state and
/// **must not be passed to an external function**
pub struct ScanFixState {
    // NOTE: See internal variables in MPS_SCAN_BEGIN
    state: mps_ss_t,
    zs: mps_word_t,
    w: mps_word_t,
    ufs: mps_word_t,
}
impl ScanFixState {
    /// Determine whether the reference needs to be fixed
    ///
    /// If this returns true, it is "interesting" to the MPS
    /// and needs to be fixed.
    ///
    /// If nothing needs to be done between `should_fix` and `fix`,
    /// you can use the convenience method `try_fix`.
    ///
    /// Corresponds to C macro `MPS_FIX1`
    #[inline(always)]
    pub unsafe fn should_fix<T>(&mut self, addr: *mut T) -> bool {
        const CHAR_BIT: usize = 8; // # of bits in a char
        let wt: mps_word_t = (1 as mps_word_t) << ((addr as mps_word_t) >> self.zs
            & (std::mem::size_of::<mps_word_t>() * CHAR_BIT - 1));
        self.ufs |= wt;
        (self.w & wt) != 0
    }
    /// Fix a reference
    ///
    /// If successful, the reference may have been moved. The scan method
    /// must store the updated reference back to the object/region being scanned.
    /// The scan method must continue to scan the block.
    ///
    /// If this returns an error, the scan method must return that immediately
    /// without fixing any further references.
    #[inline(always)]
    pub unsafe fn fix<T>(&mut self, addr: &mut *mut T) -> Result<(), mps_res_t> {
        let res = ::mps_sys::_mps_fix2(self.state, addr as *mut *mut T as *mut *mut c_void);
        if res == 0 {
            Ok(())
        } else {
            Err(res)
        }
    }
    /// Fix a reference if MPS decides it should be.
    ///
    /// Just like `fix`, this could relocate the reference
    /// so you need to store it back to the object being scanned.
    /// Errors need to be returned to the caller immediately.
    ///
    /// This is a combination of `should_fix` and `fix`
    #[inline(always)]
    pub unsafe fn try_fix<T>(&mut self, addr: &mut *mut T) -> Result<(), mps_res_t> {
        if self.should_fix(addr) {
            self.fix(addr)
        } else {
            Ok(())
        }
    }
}