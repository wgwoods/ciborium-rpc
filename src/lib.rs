// SPDX-License-Identifier: Apache-2.0

//! ciborium-rpc is a lightweight RPC protocol that uses [CBOR] (Concise Binary
//! Object Representation, [RFC 8949]) as its data format. It was inspired
//! by [JSON-RPC] and is designed to be stateless, transport-agnostic, and
//! simple, but also provide the benefits of CBOR's rich type system and
//! efficient encoding of binary data.
//!
//! The protocol is based on [JSON-RPC 2.0], with some tweaks to make the data
//! structures and overall behavior more sensible for CBOR and friendlier to
//! strongly/statically typed languages like Rust.
//!
//! This version of the protocol is experimental and subject to change without
//! warning; we make no guarantees about backwards- or forwards-compatibility
//! between experimental protocol versions.
//!
//! (We do hope to have a stable 1.0 release and a full protocol specification
//! Real Soon Now (tm), it just needs some time to mature. Consider this a
//! work-in-progress.)
//!
//! [RFC 8949]: https://datatracker.ietf.org/doc/html/rfc8949
//! [CBOR]: https://cbor.io/
//! [JSON-RPC]: https://www.jsonrpc.org/
//! [JSON-RPC 2.0]: https://www.jsonrpc.org/specification

pub mod error;
pub mod proto;
pub mod transport;

// TODO
//mod client;

// TODO
//mod server;
