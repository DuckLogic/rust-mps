#![deny(
    missing_docs, // Planned for (temporary) usage with DuckLogic
)]
#![allow(
    /*
     * This whole thing is so unsafe, I'm not even going to try
     * and document it with '# Safety' comments. It would just be insane.
     */
    clippy::missing_safety_doc,
)]
#![feature(
    concat_idents, // Used for mps_kw_arg
    negative_impls, // `!Sync` is cleaner than PhantomData
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
pub mod alloc;

pub use err::MpsError;
