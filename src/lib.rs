#![deny(
    missing_docs, // Planned for (temporary) usage with DuckLogic
)]
//! Moderately high-level bindings to the [Memory Pool System](https://www.ravenbrook.com/project/mps/).\
//!
//! There is some unsafety inherent to finding roots,
//! so using the interface typically requires unsafe code.

pub mod arena;
pub mod pools;
aa