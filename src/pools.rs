//! Supported pools

/// A pool of memory managed by the Memory Pool System
///
/// Pools are responsible for requesting memory from the arena
///
/// Some pools are automatically managed (garbage collected),
/// while others are manually managed (malloc/free)
pub trait Pool {

}