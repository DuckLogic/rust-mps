#![deny(
    missing_docs, // Planned for (temporary) usage with DuckLogic
)]
#![feature(
    concat_idents, // Used for mps_kw_arg
)]
//! Moderately high-level bindings to the [Memory Pool System](https://www.ravenbrook.com/project/mps/).\
//!
//! There is some unsafety inherent to finding roots,
//! so using the interface typically requires unsafe code.


#[macro_use]
mod err;
pub mod arena;
pub mod pools;
pub mod format;

pub use err::MpsError;
