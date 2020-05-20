use thiserror::Error;

macro_rules! handle_mps_res {
    ($res:expr) => {{
        let res: mps_sys::mps_res_t = $res;
        if res as u32 == mps_sys::MPS_RES_OK {
            Result::<(), MpsError>::Ok(())
        } else {
            Err(From::from(MpsError::from_code(res)))
        }
    }};
}

/// An error in the memory pool system
///
/// This is a high-level wrapper around `mps_res_t`
#[derive(Error, Debug)]
pub enum MpsError {
    /// MPS had an unknown failure that can't be described
    /// by any of the other error codes.
    #[error("Unknown failure")]
    Fail,
    /// MPS failured to perform system IO
    #[error("Internal IO failure")]
    Io,
    /// MPS encountered some internal limit
    #[error("Internal limit exceeded")]
    Limit,
    /// MPS ran out of memory
    #[error("Insufficient memory")]
    Memory,
    /// MPS couldn't acquire a necessary resource
    #[error("A needed resource couldn't be obtained")]
    Resource,
    /// The operation isn't currently supported
    #[error("Unsupported operation")]
    Unimplemented,
    /// The arena's maximum memory (commit limit) was excceeded
    ///
    /// This is a user-specified bound on the amount of memory that can be used
    #[error("Exceeded arena's commit limit")]
    CommitLimit,
    /// The user specified an invalid paramater
    #[error("Invalid parameter was given")]
    InvalidParam,
    /// MPS returned some other error code that isn't part
    /// of the publicly declared API
    ///
    /// If MPS returns this it's probably an error on their part.
    #[error("Unknown MPS error")]
    Unknown
}
impl MpsError {
    #[cold]
    pub(crate) fn from_code(code: mps_sys::mps_res_t) -> MpsError {
        let code = code as u32; // why?
        assert_ne!(code, mps_sys::MPS_RES_OK);
        match code {
            mps_sys::MPS_RES_FAIL => MpsError::Fail,
            mps_sys::MPS_RES_IO => MpsError::Io,
            mps_sys::MPS_RES_LIMIT => MpsError::Limit,
            mps_sys::MPS_RES_MEMORY => MpsError::Memory,
            mps_sys::MPS_RES_RESOURCE => MpsError::Resource,
            mps_sys::MPS_RES_UNIMPL => MpsError::Unimplemented,
            mps_sys::MPS_RES_COMMIT_LIMIT => MpsError::CommitLimit,
            mps_sys::MPS_RES_PARAM => MpsError::InvalidParam,
            _ => MpsError::Unknown
        }
    }
}