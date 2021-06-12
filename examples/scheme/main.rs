#![feature(
    alloc_layout_extra, // Helps computing object sizes
    try_blocks, // Used in our `from_mps_res` macro
)]
//! The MPS example scheme interpreter, ported for use with rust-mps
//!
//! See original: https://github.com/Ravenbrook/mps/blob/e493b6d/example/scheme/scheme.c
//!
//! Since this is a direct port, it falls under the same BSD license as the original.

use crate::objects::{ObjectRef, EMPTY};
use std::env::args;
use mps::arena::Arena;

/// Maximum length of a symbol
const MAX_SYMBOL: usize = 255;
/// Maximum length of an error
const MAX_ERROR: usize = 255;
/// Maximum length of a string
const MAX_STR: usize = 255;

pub type Entry = fn(env: ObjectRef, op_env: ObjectRef, rands: ObjectRef) -> ObjectRef;

pub mod objects;

pub struct SchemeContext {
    pub arena: Arena
}

pub fn main() {
    let args = args().skip(1).collect::<Vec<_>>();
    let env = ObjectRef::pair(EMPTY, EMPTY);
    let op_env = ObjectRef::pair(EMPTY, EMPTY);
    if !args.is_empty() {
        // Non-interactive file execution

    }
}