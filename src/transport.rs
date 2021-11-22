// SPDX-License-Identifier: Apache-2.0

pub use bytes::{Buf, BufMut};
pub use std::io::{Read, Write};

pub struct Transport<C: Read + Write> {
    pub channel: C,
}

impl<C> Transport<C>
where
    C: Read + Write,
{
    pub fn new(channel: C) -> Self {
        Self { channel }
    }
}

pub struct BufTransport<B: Buf + BufMut> {
    pub buffer: B,
}

impl<B> BufTransport<B>
where
    B: Buf + BufMut,
{
    pub fn new(buffer: B) -> Self {
        Self { buffer }
    }
}

pub mod cbor {
    use super::{Buf, BufMut, BufTransport, Read, Transport, Write};
    use crate::error::TransportError;
    use crate::proto::Value;
    use std::error::Error;

    pub trait CBORTransport {
        type Error: Error;
        type SendResult;
        fn send_cbor(&mut self, value: Value) -> Result<Self::SendResult, Self::Error>;
        fn read_cbor(&mut self) -> Result<Value, Self::Error>;
    }

    impl<C: Read + Write> CBORTransport for Transport<C> {
        type Error = TransportError;
        type SendResult = ();
        fn send_cbor(&mut self, value: Value) -> Result<Self::SendResult, Self::Error> {
            Ok(ciborium::ser::into_writer(&value, &mut self.channel)?)
        }
        fn read_cbor(&mut self) -> Result<Value, Self::Error> {
            Ok(ciborium::de::from_reader(&mut self.channel)?)
        }
    }
    impl<B: Buf + BufMut> CBORTransport for BufTransport<B> {
        type Error = TransportError;
        type SendResult = ();
        fn send_cbor(&mut self, value: Value) -> Result<Self::SendResult, Self::Error> {
            Ok(ciborium::ser::into_writer(
                &value,
                (&mut self.buffer).writer(),
            )?)
        }
        fn read_cbor(&mut self) -> Result<Value, Self::Error> {
            Ok(ciborium::de::from_reader((&mut self.buffer).reader())?)
        }
    }
}

pub mod simple {
    use crate::proto::{Request, Response};
    use std::error::Error;

    pub trait ClientTransport {
        type Error: Error;
        type SendResult;
        fn send_request(&mut self, request: Request) -> Result<Self::SendResult, Self::Error>;
        fn read_response(&mut self) -> Result<Response, Self::Error>;
    }

    pub trait ServerTransport {
        type Error: Error;
        type SendResult;
        fn send_response(&mut self, response: Response) -> Result<Self::SendResult, Self::Error>;
        fn read_request(&mut self) -> Result<Request, Self::Error>;
    }
}

#[cfg(test)]
mod tests {
    use super::cbor::CBORTransport;
    use super::{BufTransport, Transport};
    use crate::proto::Value;
    #[cfg(unix)]
    #[test]
    fn unix_socket_transport() {
        use std::os::unix::net::UnixStream;
        let (s1, s2) = UnixStream::pair().unwrap();
        let mut c_tr = Transport::new(s1);
        let mut s_tr = Transport::new(s2);
        let v = Value::from(vec![1, 2, 5]);
        c_tr.send_cbor(v.clone()).unwrap();
        assert_eq!(s_tr.read_cbor().unwrap(), v);
    }

    #[test]
    fn buf_transport() {
        use bytes::BytesMut;
        let mut tr = BufTransport::new(BytesMut::with_capacity(4096));
        let str_vec = vec!["one", "two", "three"];
        let v = Value::Array(str_vec.iter().map(|s| Value::from(s.to_string())).collect());
        tr.send_cbor(v.clone()).unwrap();
        assert_eq!(
            tr.buffer.len(),
            str_vec.iter().map(|s| s.len() + 1).sum::<usize>() + 1
        );
        assert_eq!(tr.read_cbor().unwrap(), v);
    }
}
