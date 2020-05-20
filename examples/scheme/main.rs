//! The MPS example scheme interpreter, ported for use with rust-mps
//!
//! See original: https://github.com/Ravenbrook/mps/blob/e493b6d/example/scheme/scheme.c

pub enum Object<'gc> {
    Pair(&'gc Object<'gc>, &'gc Object<'gc>),
    Integer(i64),
    String(String),
}