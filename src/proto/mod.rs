// SPDX-License-Identifier: Apache-2.0

/// Defines the protocol's message types and their contents.

use std::convert::TryFrom;

#[cfg(feature = "serde1")]
use serde::{Serialize, Deserialize};

#[cfg(feature = "serde1")]
pub mod v0;

// FUTURE: it'd be great if we had a v1 protocol that used CBOR tags to
// identify the message parts rather than string identifiers.
// Unfortunately, serde really has a hard time with non-string tags for enums,
// so we'll probably have to handle the message framing ourselves...

// ----- Value ----------------------------------------------------------------

// Our basic dynamic type - an arbitrary CBOR value.
pub use ciborium::value::Value;

// ----- Message Types --------------------------------------------------------

/// A Request consists of the MethodID (a string or integer), the Params to
/// pass to that method, and an optional RequestID.
/// This is usually built by a RequestBuilder.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde1", derive(Serialize, Deserialize))]
pub struct Request {
    method: MethodID,
    params: Option<Params>,
    req_id: Option<RequestID>,
}

/// A Response message has two variants: Ok and Err.
/// An Ok response contains an application-defined CBOR Value, and an Err
/// contains an [ErrorValue] describing the error that occurred.
/// Both must include the RequestID that was in the Request.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde1", derive(Serialize, Deserialize))]
pub struct Response {
    result: Result<Value, ErrorValue>,
    req_id: RequestID,
}

// ----- Data Structures ------------------------------------------------------

/// Methods can be referred to by name (String) or a numeric ID/index.
#[derive(Debug, Clone, PartialEq, Hash)]
#[cfg_attr(feature = "serde1", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde1", serde(untagged))]
pub enum MethodID {
    String(String),
    Number(u64),
}

/// A RequestID is a value that is used to identify a request so that it can
/// be matched up with its corresponding Response.
#[derive(Debug, Clone, PartialEq, Hash)]
#[cfg_attr(feature = "serde1", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde1", serde(untagged))]
pub enum RequestID {
    Number(u64),
    String(String),
    Binary(Vec<u8>),
}

/// A `Params` item holds the arguments to be passed to a remote method.
/// They can be sent in one of two forms:
///
///   Array: a simple argv-style list (`Vec<Value>`)
///   Object: key-value pairs (`Vec<(String, Value)>`)
///
/// Note that (unlike, say, Python functions) a method's arguments must either
/// be all key-value pairs or all simple values. You could have a method take
/// an Array where each Value is (Option<String>, Value) if you wanted to mix
/// keyval and non-keyval arguments, but... that's none of my business.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde1", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde1", serde(untagged))]
pub enum Params {
    Array(Vec<Value>),
    Named(Vec<(String, Value)>),
}

/// An ErrorValue is returned by the server when a Request does not complete
/// successfully.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde1", derive(Serialize, Deserialize))]
pub struct ErrorValue {
    code: i64,
    message: String,
    #[cfg_attr(feature = "serde1", serde(skip_serializing_if="Option::is_none"))]
    data: Option<Value>,
}

// ----- Useful methods for the above items -----------------------------------

macro_rules! impl_getters {
    ($(
        $type:ty { $($field:ident: $fieldtype:ty),+ $(,)? }
    ),+ $(,)?) => {
        $(
        impl $type {
            $(
            pub fn $field(&self) -> &$fieldtype {
                &self.$field
            }
            )+
        }
        )+
    };
}

impl_getters! {
    ErrorValue { code: i64, message:String, data:Option<Value> },
    Request { method: MethodID, params:Option<Params>, req_id:Option<RequestID> },
    Response { result: Result<Value,ErrorValue>, req_id:RequestID }
}

impl Params {
    pub fn is_empty(&self) -> bool {
        match self {
            Params::Array(v) => v.is_empty(),
            Params::Named(v) => v.is_empty(),
        }
    }

    /// Convert into Option<Params>, turning an empty set of Params into None.
    pub fn into_option(self) -> Option<Self> {
        if self.is_empty() {
            None
        } else {
            Some(self)
        }
    }
}

// ----- Value conversion impls for Params, RequestID, MethodID, etc ----------

use crate::error::ProtocolError;

fn to_keyval(pair: (Value, Value)) -> Result<(String, Value), ProtocolError> {
    match pair {
        (Value::Text(s), v) => Ok((s, v)),
        _ => Err(ProtocolError::InvalidKeyType),
    }
}

impl TryFrom<Value> for Params {
    type Error = ProtocolError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Array(a) => Ok(Params::Array(a)),
            Value::Map(m) => Ok(Params::Named(
                m.into_iter().map(to_keyval).collect::<Result<_, _>>()?,
            )),
            _ => Err(Self::Error::InvalidParamType),
        }
    }
}

impl TryFrom<Value> for RequestID {
    type Error = ProtocolError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Integer(i) => match u64::try_from(i) {
                Ok(u) => Ok(u.into()),
                Err(_) => Err(Self::Error::InvalidRequestID),
            },
            Value::Text(s) => Ok(s.into()),
            Value::Bytes(b) => Ok(b.into()),
            _ => Err(Self::Error::InvalidRequestID),
        }
    }
}

impl TryFrom<Value> for MethodID {
    type Error = ProtocolError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Integer(i) => match u64::try_from(i) {
                Ok(u) => Ok(u.into()),
                Err(_) => Err(Self::Error::InvalidMethodID),
            },
            Value::Text(s) => Ok(s.into()),
            _ => Err(Self::Error::InvalidMethodID)
        }
    }
}

impl From<Params> for Value {
    fn from(params: Params) -> Self {
        match params {
            Params::Array(v) => Self::Array(v),
            Params::Named(nv) => Self::Map(nv.into_iter().map(|(n, v)| (n.into(), v)).collect()),
        }
    }
}

impl From<RequestID> for Value {
    fn from(r: RequestID) -> Self {
        match r {
            RequestID::Binary(b) => Value::Bytes(b.into()),
            RequestID::Number(i) => Value::Integer(i.into()),
            RequestID::String(s) => Value::Text(s.into()),
        }
    }
}

impl From<MethodID> for Value {
    fn from(m: MethodID) -> Self {
        match m {
            MethodID::Number(i) => Value::Integer(i.into()),
            MethodID::String(s) => Value::Text(s.into()),
        }
    }
}

macro_rules! implfrom {
    ($($enum:ident::$variant:ident <= $fromtype:ty),+ $(,)?) => {
        implfrom! { $( $fromtype => $enum::$variant, )+ }
    };
    ($($fromtype:ty => $enum:ident::$variant:ident),+ $(,)?) => {
        $(
            impl From<$fromtype> for $enum {
                #[inline]
                fn from(value: $fromtype) -> Self {
                    Self::$variant(value.into())
                }
            }
        )+
    };
}

implfrom! {
    Vec<Value> => Params::Array,
    Vec<(String, Value)> => Params::Named,

    u64 => MethodID::Number,
    u32 => MethodID::Number,
    u16 => MethodID::Number,
    u8 => MethodID::Number,

    String => MethodID::String,
    &str => MethodID::String,

    u64 => RequestID::Number,
    u32 => RequestID::Number,
    u16 => RequestID::Number,
    u8 => RequestID::Number,

    String => RequestID::String,
    &str => RequestID::String,

    Vec<u8> => RequestID::Binary,
}

