// SPDX-License-Identifier: Apache-2.0

// We're using serde to define a single RPCMsg type and then implement
// ClientTransport/ServerTransport in terms of serializing/deserializing
// to/from RPCMsg.
#![cfg(feature = "serde1")]
use serde::{Deserialize, Serialize};

use ciborium::tag::Required;
use std::convert::{TryFrom, TryInto};

use super::{ErrorValue, MethodID, Params, RequestID, Value, Request, Response};
use crate::error::{ProtocolError, TransportError};
use crate::transport::simple::{ClientTransport, ServerTransport};
use crate::transport::{Buf, BufMut, Read, Write};
use crate::transport::{BufTransport, Transport};

// An arbitrary magic number / tag ID to identify RPC V0 requests
const RPCV0: u64 = 4036988077;

/// This defines how we serialize/deserialize the Request struct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "crate::proto::Request")]
struct RequestMsg {
    #[serde(rename = "fn")]
    method: MethodID,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "args")]
    params: Option<Params>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "id")]
    req_id: Option<RequestID>,
}

/// This defines how we serialize/deserialize the Result inside a Response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "core::result::Result")]
enum ResultMsg<T, E> {
    #[serde(rename = "ok")]
    Ok(T),
    #[serde(rename = "err")]
    Err(E),
}

/// This is how we serialize/deserialize the Response struct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "crate::proto::Response")]
struct ResponseMsg {
    #[serde(flatten, with = "ResultMsg")]
    result: Result<Value, ErrorValue>,
    #[serde(rename = "id")]
    req_id: RequestID,
}

/// The Msg enum encapsulates all well-formatted RPC message contents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum Msg {
    Request(#[serde(with = "RequestMsg")] crate::proto::Request),
    Response(#[serde(with = "ResponseMsg")] crate::proto::Response),
}

/// RPCMsg is the toplevel type for this version of the protocol.
///
/// Every RPC message is tagged with CBOR tag [RPCV0] so we can identify it,
/// and then contains either a Request or a Response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RPCMsg(Required<Msg, RPCV0>);

//
impl RPCMsg {
    fn from_reader(reader: &mut impl Read) -> Result<Self, TransportError> {
        Ok(ciborium::de::from_reader(reader)?)
    }
    fn into_writer(&self, writer: &mut impl Write) -> Result<(), TransportError> {
        Ok(ciborium::ser::into_writer(self, writer)?)
    }
    fn from_buf(buf: &mut impl Buf) -> Result<Self, TransportError> {
        Self::from_reader(&mut buf.reader())
    }
    fn into_buf_mut(&self, buf_mut: &mut impl BufMut) -> Result<(), TransportError> {
        self.into_writer(&mut buf_mut.writer())
    }
}

// ----- Conversions to/from RPCMsg

impl From<Request> for RPCMsg {
    fn from(r: Request) -> Self {
        RPCMsg(Required(Msg::Request(r)))
    }
}
impl From<Response> for RPCMsg {
    fn from(r: Response) -> Self {
        RPCMsg(Required(Msg::Response(r)))
    }
}
impl TryFrom<RPCMsg> for Request {
    type Error = ProtocolError;
    fn try_from(msg: RPCMsg) -> Result<Self, Self::Error> {
        match msg.0 .0 {
            Msg::Request(r) => Ok(r),
            Msg::Response(_) => Err(ProtocolError::UnexpectedMessage),
        }
    }
}
impl TryFrom<RPCMsg> for Response {
    type Error = ProtocolError;
    fn try_from(msg: RPCMsg) -> Result<Self, Self::Error> {
        match msg.0 .0 {
            Msg::Request(_) => Err(ProtocolError::UnexpectedMessage),
            Msg::Response(r) => Ok(r),
        }
    }
}

// Here we implement ClientTransport/ServerTransport using the stuff above, so
// our generic Transport<C> and BufTransport<B> can transport RPCMsg items.

impl<C: Read + Write> ClientTransport for Transport<C> {
    type Error = TransportError;
    type SendResult = ();
    fn read_response(&mut self) -> Result<Response, Self::Error> {
        Ok(RPCMsg::from_reader(&mut self.channel)?.try_into()?)
    }
    fn send_request(&mut self, request: Request) -> Result<Self::SendResult, Self::Error> {
        Ok(RPCMsg::from(request).into_writer(&mut self.channel)?)
    }
}
impl<C: Read + Write> ServerTransport for Transport<C> {
    type Error = TransportError;
    type SendResult = ();
    fn read_request(&mut self) -> Result<Request, Self::Error> {
        Ok(RPCMsg::from_reader(&mut self.channel)?.try_into()?)
    }
    fn send_response(&mut self, response: Response) -> Result<Self::SendResult, Self::Error> {
        Ok(RPCMsg::from(response).into_writer(&mut self.channel)?)
    }
}
impl<B: Buf + BufMut> ClientTransport for BufTransport<B> {
    type Error = TransportError;
    type SendResult = ();
    fn read_response(&mut self) -> Result<Response, Self::Error> {
        Ok(RPCMsg::from_buf(&mut self.buffer)?.try_into()?)
    }
    fn send_request(&mut self, request: Request) -> Result<Self::SendResult, Self::Error> {
        Ok(RPCMsg::from(request).into_buf_mut(&mut self.buffer)?)
    }
}
impl<B: Buf + BufMut> ServerTransport for BufTransport<B> {
    type Error = TransportError;
    type SendResult = ();
    fn read_request(&mut self) -> Result<Request, Self::Error> {
        Ok(RPCMsg::from_buf(&mut self.buffer)?.try_into()?)
    }
    fn send_response(&mut self, response: Response) -> Result<Self::SendResult, Self::Error> {
        Ok(RPCMsg::from(response).into_buf_mut(&mut self.buffer)?)
    }
}

#[cfg(test)]
mod tests {
    use crate::transport::simple::{ClientTransport, ServerTransport};
    use crate::transport::cbor::CBORTransport;
    use crate::transport::BufTransport;
    use super::{Request, Response};
    use crate::proto::{ErrorValue, Params, Value};
    use bytes::BytesMut;
    macro_rules! params {
        ($($v:expr),+ $(,)?) => {
            Params::Array(vec![$(
                Value::from($v),
            )+
            ])
        };
    }
    #[test]
    fn encode_request() {
        let mut tr = BufTransport::new(BytesMut::with_capacity(4096));
        let mut req = Request {
            method: "hello".into(),
            params: Some(params!["one", 2, "three"]),
            req_id: Some(42u32.into()),
        };
        tr.send_request(req.clone()).unwrap();
        assert!(tr.buffer.len() <= 38);
        let req2: Request = tr.read_request().unwrap();
        println!("req: {:?}", req2);
        assert_eq!(req, req2);
        req.params = None;
        tr.send_request(req.clone()).unwrap();
        let req2: Request = tr.read_request().unwrap();
        println!("req: {:?}", req2);
        assert_eq!(req, req2);
        req.req_id = None;
        tr.send_request(req.clone()).unwrap();
        let req2: Request = tr.read_request().unwrap();
        println!("req: {:?}", req2);
        assert_eq!(req, req2);
    }
    #[test]
    fn encode_response() {
        let mut tr = BufTransport::new(BytesMut::with_capacity(4096));
        let mut resp = Response {
            result: Ok("yay".into()),
            req_id: 42u32.into(),
        };
        tr.send_response(resp.clone()).unwrap();
        let resp2: Response = tr.read_response().unwrap();
        println!("resp: {:?}", resp2);
        assert_eq!(resp, resp2);
        resp.result = Err(ErrorValue {
            code: 418,
            message: "I'm a teapot".into(),
            data: None,
        });
        tr.send_response(resp.clone()).unwrap();
        println!("len: {:?}", tr.buffer.len());
        println!("msg: {:?}", tr.buffer);
        let val = tr.read_cbor().unwrap();
        println!("val: {:?}", val);
        tr.send_response(resp.clone()).unwrap();
        let resp2: Response = tr.read_response().unwrap();
        println!("resp: {:?}", resp2);
        assert_eq!(resp, resp2);
    }
}