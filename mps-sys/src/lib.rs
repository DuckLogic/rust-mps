#![allow(
    // C libraries have different naming conventions
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    // Most of the code here is auto-generated
    clippy::missing_safety_doc
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
            val: $crate::mps_arg_val::from($val)
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
pub type mps_arg_val = mps_arg_s__bindgen_ty_1;
macro_rules! mps_arg_val_from {
    ($($src_type:ty $(|$src:ident| $cast:expr)? => $field_name:ident),*) => {
        $(impl From<$src_type> for mps_arg_val {
            #[inline]
            fn from(val: $src_type) -> Self {
                unsafe {
                    let mut res = std::mem::zeroed::<mps_arg_val>();
                    $(let val = {
                        let $src = val;
                        $cast
                    };)?
                    *res.$field_name.as_mut() = val;
                    res
                }
            }
        })*
    };
}
mps_arg_val_from!(
    bool |b| b as mps_bool_t => b,
    usize => size,
    f64 => d,
    mps_fmt_t => format,
    mps_fmt_scan_t => fmt_scan,
    mps_fmt_skip_t => fmt_skip,
    mps_fmt_fwd_t => fmt_fwd,
    mps_fmt_pad_t => fmt_pad
    // mps_fmt_class_t => fmt_class
);

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
