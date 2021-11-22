// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("invalid method id")]
    InvalidMethodID,
    #[error("invalid request id")]
    InvalidRequestID,
    #[error("invalid type for params")]
    InvalidParamType,
    #[error("non-string key in params")]
    InvalidKeyType,
    #[error("not an RPC message")]
    InvalidMessage,
    #[error("incorrect message type")]
    UnexpectedMessage,
}

#[derive(Error, Debug)]
pub enum TransportError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("protocol error: {0}")]
    Proto(#[from] ProtocolError),

    #[error("encode error: {0}")]
    Encode(String),

    #[error("decode error{}: {msg}",
        .pos.map(|p| format!(" at pos {}", p)).unwrap_or("".into())
    )]
    Decode { msg: String, pos: Option<usize> },
}

impl<E> From<ciborium::ser::Error<E>> for TransportError
where TransportError: From<E>
{
    fn from(err: ciborium::ser::Error<E>) -> Self {
        use ciborium::ser::Error::*;
        match err {
            Io(e) => e.into(),
            Value(s) => TransportError::Encode(s),
        }
    }
}

impl <E> From<ciborium::de::Error<E>> for TransportError
where TransportError: From<E>
{
    fn from(err: ciborium::de::Error<E>) -> Self {
        use ciborium::de::Error::*;
        match err {
            Io(e) => TransportError::from(e),
            Semantic(pos, msg) => TransportError::Decode { msg, pos },
            Syntax(pos) => TransportError::Decode {
                msg: "syntax error".into(), pos: Some(pos)
            },
            RecursionLimitExceeded => TransportError::Decode {
                msg: "recursion limit exceeded".into(), pos: None
            }
        }
    }
}