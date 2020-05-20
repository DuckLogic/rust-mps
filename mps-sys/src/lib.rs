#![allow(
    // FFI bindings must mirror raw types
    non_camel_case_types,
)]
use std::os::raw::c_int;

/// A raw result code from MPS
pub type mps_res_t = c_int;
/*
 * Internal result codes
 * Must mirror order in `_mps_RES_ENUM` in `code/mps.h`
 */
/// success (always zero)
pub const MPS_RES_OK: mps_res_t = 0;
/// unspecified failure
pub const MPS_RES_FAIL: mps_res_t = 1;
/// unable to obtain resources
pub const MPS_RES_RESOURCE: mps_res_t = 2;
/// unable to obtain memory
pub const MPS_RES_MEMORY: mps_res_t = 3;
/// limitation reached
pub const MPS_RES_LIMIT: mps_res_t = 4;
/// unimplemented facility
pub const MPS_RES_UNIMPL: mps_res_t = 5;
/// system I/O error
pub const MPS_RES_IO: mps_res_t = 6;
/// arena commit limit exceeded
pub const MPS_COMMIT_LIMIT: mps_res_t = 7;
/// illegal user parameter value
pub const MPS_PARAM: mps_res_t = 8;

pub mod arena;