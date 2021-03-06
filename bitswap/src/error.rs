use thiserror::Error;
use futures::channel::{mpsc, oneshot};

#[derive(Debug, Error)]
pub enum BitswapError {
    #[error("Error while decoding bitswap message: {0}")]
    ProtobufError(#[from] prost::DecodeError),
    #[error("Closing")]
    Closing,
    #[error("Error Cid {0}")]
    Cid(#[from] libipld::cid::Error),
    #[error("Timeout")]
    Timeout,
    #[error("Error sending {0}")]
    Send(#[from] mpsc::SendError),
    #[error("Cancelled oneshot {0}")]
    Cancel(#[from] oneshot::Canceled),
}
//
// impl From<mpsc::SendError> for BitswapError {
//     fn from(_: mpsc::SendError) -> Self {
//         BitswapError::Send
//     }
// }