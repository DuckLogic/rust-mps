//! MPS uses [allocation points](https://www.ravenbrook.com/project/mps/master/manual/html/topic/allocation.html#allocation-points)
//! to provide fast, inline, nearly lock-free allocation.
//!
//! It allows code to allocate without calling an allocation function. This is vital for performance
//! in languages that allocate many small objects.
//!
//! Here is a description of the allocation pont protocol,
//! slightly modified to fit with the Rust bindings:
//! ## Allocation Point Protocol
//! This protocol is designed to work with incremental garbage collection and multiple threads,
//! where between any two instructions in the client program, the MPS may run part of a garbage collection,
//! move blocks in memory, rewrite pointers, and reclaim space. In order to reliably handle this,
//! the allocation point protocol consists of (at least) two steps: a *reserve*, followed by a *commit*.
//!
//! When the client program is initializing a newly allocated object, you can think of it as being
//! "in a race" with the MPS. Until the object is initialized, the MPS cannot manage it in the usual way:
//! in particular, it cannot ensure that the new object remains correct if other objects move during
//! its initialization.
//! So if other objects *do* move, the MPS tells the client program that it has "lost the race":
//! the partially-initialized object may be invalid, and the client must initialize it again from scratch.
//!
//! The allocation point protocol is as follows:
//! 1. Call [AllocationPoint::reserve] to reserve a block of memory on an allocation point.
//!    The size of the block must be a multiple of the alignment of the pool in which the protocol was created.
//!    If the function returns an [MpsError], the block cannot be reserved (this can happen if out of memory).
//! 2. Initialize the block. During this step, the block must not be referenced by an exact reference,
//!    and references stored in it must not be followed.
//!    The block need not be initialized completely, but if the pool has an object format,
//!    than by the end of this step, the block must be capable of being past to the format's scan and skip methods.
//! 3. Call [AllocationPoint::commit] to attempt to commit the object to the care of the MPS.
//!    - If [AllocationPoint::commit] returns true, this means the object is valid, and is now under
//!      the management of the MPS. The client program may rely on references sotred in this object,
//!      and may store references to the new object in other objects.
//!    - If [AllocationPoint::commit] returns false, this means the block is invalid. Usually, clients
//!      go back to step 1 and re-reserve and re-intialize it, but other courses of action are permitted.
//!    - NOTE: In this case, the reason the block is invalid is because a flip took palce after the call
//!      to [AllocationPoint::reserve] and before the call to [AllocationPoint::commit]. This means that
//!      references in the block may point to the old location of blocks that have moved.
//!
//! ### Example in (unsafe) Rust
//! ````rust,no_run
//! # use std::alloc::Layout;
//! # use std::{ptr, mem};
//! # use mps::alloc::AllocationPoint;
//! # use mps_sys::mps_addr_t;
//! # pub struct ExampleObject { val: usize }
//! # const POOL_ALIGNMENT: usize = mem::align_of::<usize>();
//! let mut obj: *mut ExampleObject = ptr::null_mut();
//! let layout = Layout::new::<ExampleObject>();
//! // Alignment must be at least as big as the pool's alignment
//! debug_assert!(layout.align() >= POOL_ALIGNMENT);
//! # let ap: &AllocationPoint = todo!("Create an allocation point");
//! loop {
//!     unsafe {
//!         let res = ap.reserve(layout.size()).expect("Allocation failed") as *mut ExampleObject;
//!         /* `res` is now an ambiguous reference to the reserved block */
//!         // Initialize
//!         (*res).val = 5;
//!         if ap.commit(res as mps_addr_t, layout.size()) {
//!             // Successfull allocation
//!             obj = res;
//!             break;
//!         }
//!     }
//! }
//! // Successful allocation
//! assert_eq!(unsafe { (*obj).val }, 5);
//! ````
use mps_sys::{mps_ap_t, mps_addr_t};

use crate::err::MpsError;

/// An allocation point.
///
/// This is represented as a pointer to a [mps_ap_s](::mps_sys::mps_ap_s). This
/// representation can be safely relied upon for FFI. In other words, it's safe to transmute
/// back and forth.
#[repr(transparent)]
pub struct AllocationPoint {
    raw: mps_ap_t
}
impl AllocationPoint {
    /// Create an allocation point from the specified raw pointer.
    ///
    /// This function takes ownership of the allocation point.
    ///
    /// Undefined behavior if the allocation point is invalid.
    #[inline(always)]
    pub const unsafe fn from_raw(raw: mps_ap_t) -> AllocationPoint {
        AllocationPoint { raw }
    }
    /// Get the raw pointer to the underlying allocation point
    #[inline(always)]
    pub const fn as_raw(&self) -> mps_ap_t {
        self.raw
    }
    /// Reserve a block of memory from this allocation point.
    ///
    /// The size of the block to allocate must be a multiple of the alignment of the pool
    /// (or the pool's object format if it has one).
    ///
    /// Returns `Ok` if the block was reserved successfully, or [MpsError] if it was not.
    ///
    /// The reserved block may be initialized but must not otherwise be used.
    ///
    /// Until it has been committed via a successful call to [AllocationPoint::commit],
    /// the reserved block may be:
    /// - initialized
    /// - referenced by an ambiguous reference
    /// but:
    /// - it must not be referenced by an exact reference
    /// - references stored in it must not be followed
    /// - it is not scanned, moved, or protected (even if it belongs to a pool with those features)
    ///
    /// ## Safety
    /// - Undefined behavior if the size is not properly aligned.
    /// - Undefined behavior if the resulting pointer is used improperly.
    ///   It must be initialized before any further use (see module docs).
    #[inline(always)]
    pub unsafe fn reserve(&self, size: usize) -> Result<mps_addr_t, MpsError> {
        /*
         * C impl: https://github.com/Ravenbrook/mps/blob/e198a504f3ba2197686c55/code/mps.h#L614
         * See also relevant docs:
         * https://www.ravenbrook.com/project/mps/master/manual/html/topic/allocation.html#allocation-point-implementation
         */
        let alloc = (*self.raw).alloc;
        let next = alloc.wrapping_add(size);
        if next > alloc && next <= (*self.raw).limit {
            (*self.raw).alloc = next;
            Ok((*self.raw).init)
        } else {
            self.fill(size)
        }
    }
    /// Commit a previously reserved block on an allocation point.
    ///
    /// If [commit](AllocationPoint::commit) returns true, the block was successfully committed,
    /// which means that the client program may use it, create references to it, and rely on references from it.
    /// It also means that the MPS may scan it, move it, protect it, or reclaim it.
    ///
    /// If [commit](AllocationPoint::commit) returns false, the block was not committed. It is very rare for this to
    /// happen and only occurs if there was a flip between the call to [AllocationPoint::reserve] and the call to `commit`.
    ///
    /// ## Safety
    /// - The pointer must have been previously given by this allocation point's [reserve](AllocationPoint::reserve)
    ///   method, and must have been initialized consistent with the allocation point protocol (as described in the module docs).
    /// - The size must match the allocated size
    /// - The memory must be fully initialized, and ready to be scanned by the MPS.
    #[inline(always)]
    pub unsafe fn commit(&self, p: mps_addr_t, size: usize) -> bool {
        // https://github.com/Ravenbrook/mps/blob/e198a504f3ba2197686c55e048996/code/mps.h#L640
        (*self.raw).init = (*self.raw).alloc;
        if !(*self.raw).limit.is_null() {
            true
        } else {
            self.trip(p, size)
        }
    }
    /// Rserve a block of memory on an allocation point,
    /// when the inline buffer has insufficient space.
    ///
    /// Corresponds to C function [mps_ap_fill](https://www.ravenbrook.com/project/mps/master/manual/html/topic/allocation.html#c.mps_ap_fill)
    #[cold]
    #[inline(always)]
    unsafe fn fill(&self, size: usize) -> Result<mps_addr_t, MpsError> {
        let mut res: mps_addr_t = std::ptr::null_mut();
        match ::mps_sys::mps_ap_fill(&mut res, self.raw, size) {
            0 => Ok(res),
            code => Err(MpsError::from_code(code))
        }
    }
    /// Tests whether a reserved block was successfully committed when an allocation point was trapped.
    ///
    /// Corresponds to C function [mps_ap_trip](https://www.ravenbrook.com/project/mps/master/manual/html/topic/allocation.html#c.mps_ap_trip)
    #[inline(always)]
    #[cold]
    unsafe fn trip(&self, ptr: mps_addr_t, size: usize) -> bool {
        ::mps_sys::mps_ap_trip(self.raw, ptr, size) != 0
    }
}