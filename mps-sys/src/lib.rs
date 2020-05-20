#![allow(
    // C libraries have different naming conventions
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals
)]
#![feature(
    concat_idents, // Used for mps_kw_arg
)]

//! Automatically generated bindings to MPS
include!(concat!(env!("OUT_DIR"), "/mps_auto.rs"));

/// An unsigned word-sized integer
pub type mps_word_t = usize;

#[macro_export]
macro_rules! mps_kw_arg {
    ($key:ident => $val:expr) => {{
        use $crate::*;
        mps_arg_s {
            key: &concat_idents!(_mps_key_, $key),
            val: std::mem::transmute($val)
        }
    }};
}
#[inline]
pub unsafe fn mps_args_end() -> mps_arg_s {
    mps_arg_s {
        key: &_mps_key_ARGS_END,
        val: std::mem::zeroed()
    }
}

/// Rust imitation of `MPS_ARGS_BEGIN/END` marcos
///
/// Very unsafe internally!
#[macro_export]
macro_rules! mps_kw_args {
    ($($key:ident => $val:expr),*) => {{
        arrayvec::ArrayVec::from([
            $(mps_kw_arg!($key => $val),)*
            $crate::mps_args_end()
        ])
    }};
}