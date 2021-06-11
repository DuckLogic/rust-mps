//! All the supported MPS arenas
use mps_sys::*;
use arrayvec::ArrayVec;
use crate::err::MpsError;

/// A MPS Arena, for allocating raw memory from the operating system
///
/// Generally you want to use a ["Virtual memory" arena](https://www.ravenbrook.com/project/mps/master/manual/html/topic/arena.html#virtual-memory-arenas),
/// to use the OS's virtual memory system
pub struct Arena {
    raw: mps_arena_t
}
impl Arena {
    #[inline]
    pub(crate) fn as_raw(&self) -> mps_arena_t {
        self.raw
    }
    /// The number of collections in which objects might have been moved.
    ///
    /// If you're using a non-moving pool, this will return zero.
    #[inline]
    pub fn moved_collections(&self) -> usize {
        unsafe { mps_collections(self.raw) }
    }
    /// The current commit limit for this arena (in bytes)
    ///
    /// For a virtual memory arena, this is the maximum amout
    /// of memory this arena will map to RAM.
    #[inline]
    pub fn commit_limit(&self) -> usize {
        unsafe { mps_arena_commit_limit(self.raw) }
    }
    /// Attempt to set the commit limit to the specified value
    ///
    /// The commit limit cannot be set to a value that is lower
    /// than the number of bytes that the MPS is using.
    #[inline]
    pub fn set_commit_limit(&self, limit: usize) -> Result<(), MpsError> {
        unsafe {
            handle_mps_res!(mps_arena_commit_limit_set(self.raw, limit))
        }
    }
    /// The total committed memory for an arena
    ///
    /// For a virtual memory arena, this is the amount of memory mapped
    /// to RAM by the operating system’s virtual memory interface.
    ///
    /// The committed memory is generally larger than the sum
    /// of the sizes of the allocated blocks.
    ///
    /// This is due to:
    /// 1. Internal MPS memory usage
    /// 2. Operating systems generally restrict programs to allocate multiples of the page size
    /// 3. Spare committed memory
    ///
    /// The amount of committed memory is a good measure of how much
    /// virtual memory resource (“swap space”) the MPS is using
    /// from the operating system.
    ///
    /// If you want to know the total memory usage you should probably subtract
    /// `spare_committed`
    #[inline]
    pub fn committed(&self) -> usize {
        unsafe { mps_arena_committed(self.raw) }
    }
    /// Return the maximum time, in seconds,
    /// that operations within the arena may pause the client program for.
    ///
    /// This is an advisory limit.
    /// See [Arena::set_pause_time] for more details.
    #[inline]
    pub fn pause_time(&self) -> f64 {
        unsafe { mps_arena_pause_time(self.raw) }
    }
    /// Set the maximum time, in seconds, that operations within
    /// an arena may pause the client program for.
    ///
    /// The MPS makes more efficient use of processor time when it is allowed longer pauses, up to the maximum
    /// time it takes to collect the entire arena.
    ///
    /// This is an advisory or best-effort limit.
    /// There is no hard guarantee that the arena
    /// will complete in the .
    ///
    /// See [mps_arena_pause_time_set](https://www.ravenbrook.com/project/mps/master/manual/html/topic/arena.html#c.mps_arena_pause_time_set)
    /// for more details.
    #[inline]
    pub fn set_pause_time(&self, pause_time: f64) {
        assert!(pause_time >= 0.0);
        unsafe { mps_arena_pause_time_set(self.raw, pause_time) }
    }
    /// The current spare commit limit for this arena,
    /// as a proportion of total committed size.
    ///
    /// This is the fraction of memory that can be committed (reserved)
    /// that is not currently in use by the program.
    ///
    /// If memory usage is lower than this fraction, the MPS
    /// will return some back to the operating system.
    #[inline]
    pub fn spare_limit(&self) -> f64 {
        unsafe { mps_arena_spare(self.raw) }
    }
    /// The amount of committed memory that is "spare"
    /// and is not currently being used by MPS or the client program.
    ///
    /// It is used by the arena to avoid calling the operating system
    /// to repeatedly map virtual memory. The proportion of spare memory
    /// should always fall below the limit given by `spare_limit`.
    #[inline]
    pub fn spare_committed(&self) -> usize {
        unsafe { mps_arena_spare_committed(self.raw) }
    }
    /// Request the collector to begin garbage collection.
    ///
    /// This will return quickly, without blocking until completion.
    /// Contrast this to [Arena::full_collection]
    ///
    /// Returns `Ok(())` if collection successfully
    /// begins and an error if it is not.
    /// Generally, errors from this method are non fatal (and can be safely ignored).
    #[inline]
    pub fn begin_collection(&self) -> Result<(), MpsError> {
        unsafe {
            handle_mps_res!(mps_arena_start_collect(self.raw))
        }
    }
    /// Begin a full collection, blocking until completion
    ///
    /// Contrast with [Arena::begin_collection], which asynchronously
    /// requests a collection, without blocking until completion.
    #[inline]
    pub fn full_collection(&self) {
        unsafe { mps_arena_collect(self.raw); }
    }

}
impl Drop for Arena {
    fn drop(&mut self) {
        unsafe {
            // NOTE: Everything else must be destroyed first
            mps_arena_destroy(self.raw);
        }
    }
}
/// MPS is thread safe
unsafe impl Send for Arena {}
/// MPS is thread safe. I think this is pretty much true of all operations
///
/// <https://www.ravenbrook.com/project/mps/master/manual/html/design/thread-safety.html>
unsafe impl Sync for Arena {}

/// An arena that uses the operating system's virtual memory system (`mmap`)
/// to allocate internal memory.
///
/// This is currently the only supported arena class.
///
/// This gives MPS the maximum flexibility on where to locate memory
/// and it can have many more "virtual" addresses than are physically in use.
pub struct VirtualMemoryArenaClass {
    raw: mps_arena_class_t
}
impl VirtualMemoryArenaClass {
    /// Return the arena class for a virtual memory arena.
    ///
    /// This is a global singleton. It lives forever
    pub fn get() -> VirtualMemoryArenaClass {
        VirtualMemoryArenaClass { raw: unsafe { mps_arena_class_vm() } }
    }
    /// Create a builder to change settings on the arena
    pub fn builder(&self) -> VirtualMemoryArenaBuilder {
        VirtualMemoryArenaBuilder {
            class: self.raw, // NOTE: Global singleton
            arena_size: None,
            commit_limit: None,
            spare: None,
            pause_time: None
        }
    }
}
/// Builds a virtual memory arena
///
/// This can be used to change the created arena's settings
pub struct VirtualMemoryArenaBuilder {
    class: mps_arena_class_t,
    /// The initial amount of address space that the arena will reserve
    ///
    /// As of this writing, this defaults to 256 MB
    pub arena_size: Option<usize>,
    /// the *maximum* amount of memory that the arena will reserve
    ///
    /// As of this writing, this defaults to `usize::max_value()`
    pub commit_limit: Option<usize>,
    /* NOTE: Omit MPS_KEY_ARENA_GRAIN_SIZE */
    /// The maximum portion of committed memory that the arena will
    /// retain for future allocations.
    ///
    /// If the percentage of spare (unused) memory is greater than this,
    /// the arena will return some back to the operating system.
    pub spare: Option<f64>,
    /// The maximum time in seconds that arena operations may pause the
    /// client for.
    ///
    /// See [mps_arena_pause_time_set](https://www.ravenbrook.com/project/mps/master/manual/html/topic/arena.html#c.mps_arena_pause_time_set)
    pub pause_time: Option<f64>,
}
impl VirtualMemoryArenaBuilder {
    /// Attempt to create a virtual memory arena with the current settings,
    /// returning an error on failure
    pub fn build(self) -> Result<Arena, MpsError> {
        let VirtualMemoryArenaBuilder { class, arena_size,
            commit_limit, spare, pause_time } = self;
        let mut kws: ArrayVec<_, 5> = ArrayVec::new();
        unsafe {
            if let Some(size) = arena_size {
                kws.push(mps_kw_arg!(ARENA_SIZE => size));
            }
            if let Some(commit_limit) = commit_limit {
                kws.push(mps_kw_arg!(COMMIT_LIMIT => commit_limit));
            }
            if let Some(spare) = spare {
                assert!((0.0..=1.0).contains(&spare), "Invalid spare: {}", spare);
                kws.push(mps_kw_arg!(SPARE => spare));
            }
            if let Some(pause_time) = pause_time {
                assert!(pause_time >= 0.0, "Invalid pause time: {}", pause_time);
                kws.push(mps_kw_arg!(PAUSE_TIME => pause_time));
            }
            kws.push(mps_args_end());
            let mut out: mps_arena_t = std::ptr::null_mut();
            handle_mps_res!(mps_arena_create_k(
                &mut out, class, kws.as_mut_ptr()
            ))?;
            assert!(!out.is_null());
            Ok(Arena { raw: out })
        }
    }
}
