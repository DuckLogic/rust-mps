#![feature(
    alloc_layout_extra, // Helps computing object sizes
)]
//! The MPS example scheme interpreter, ported for use with rust-mps
//!
//! See original: https://github.com/Ravenbrook/mps/blob/e493b6d/example/scheme/scheme.c

use crate::objects::{ObjectRef, EMPTY};
use std::env::args;

/// Maximum length of a symbol
const MAX_SYMBOL: usize = 255;
/// Maximum length of an error
const MAX_ERROR: usize = 255;
/// Maximum length of a string
const MAX_STR: usize = 255;

pub type Entry = fn(env: ObjectRef, op_env: ObjectRef, rands: ObjectRef) -> ObjectRef;

pub mod objects;

pub fn main() {
    let args = args().skip(1).collect::<Vec<_>>();
    let env = ObjectRef::pair(EMPTY, EMPTY);
    let op_env = ObjectRef::pair(EMPTY, EMPTY);
    if !args.is_empty() {
        // Non-interactive file execution

    }
}