use jsonrpsee::{IntoResponse, types::ErrorObjectOwned};
use serde::Serialize;
use serde_json::Value;

pub use jsonrpsee::types::error::ErrorCode;

/// An RPC error response. [See documentation](https://www.jsonrpc.org/specification#response_object).
/// for addtional information.
#[derive(Debug, Clone)]
pub struct RpcError {
    /// Error code.
    pub code: ErrorCode,

    /// Message describing error.
    pub message: String,

    /// Optional serialisable data to include with the error.
    pub data: Option<Value>,
}

/// Convert into [`jsonrpsee::types::ErrorObjectOwned`].
impl From<RpcError> for ErrorObjectOwned {
    fn from(rpc_error: RpcError) -> Self {
        Self::from(&rpc_error)
    }
}

/// Convert into [`jsonrpsee::types::ErrorObjectOwned`].
impl From<&RpcError> for ErrorObjectOwned {
    fn from(rpc_error: &RpcError) -> Self {
        Self::owned(
            rpc_error.code.code(),
            &rpc_error.message,
            rpc_error.data.clone(),
        )
    }
}

/// Allow for [`RpcError`] to directly be returned from handlers.
impl IntoResponse for RpcError {
    type Output = <ErrorObjectOwned as IntoResponse>::Output;

    fn into_response(self) -> jsonrpsee::ResponsePayload<'static, Self::Output> {
        ErrorObjectOwned::from(self).into_response()
    }
}

impl Serialize for RpcError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ErrorObjectOwned::from(self).serialize(serializer)
    }
}
