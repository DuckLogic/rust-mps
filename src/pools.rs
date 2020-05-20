//! Supported pools

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
/// See MPS documentation on howto best choose a pool class.
pub unsafe trait Pool<'a> {}

/// A pool that supports automatically managing memory
pub trait AutomaticPool<'a>: Pool<'a> {}
