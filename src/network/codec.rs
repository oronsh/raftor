use byteorder::{BigEndian, ByteOrder};
use bytes::{BufMut, BytesMut};
use tokio::codec::{Decoder, Encoder};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use serde_json as json;
use std::fmt::Debug;
use actix_raft::{
    NodeId,
    messages::{
        AppendEntriesRequest,
        AppendEntriesResponse,
        InstallSnapshotRequest,
        InstallSnapshotResponse,
        VoteRequest,
        VoteResponse,
    }
};

use crate::network::{Request, ReqTable};

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeRequest {
    id: String,
    payload: NodePayload,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeResponse {
    id: String,
    payload: NodePayload,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum NodePayload {
    Ping,
    Join(NodeId),
    AppendEntriesRequest,
    InstallSnapshotRequest,
    VoteRequest,
    Joined,
    VoteResponse,
    AppendEntriesResponse,
    InstallSnapshotResponse,
}



pub struct NodeCodec;

// Client -> Server transport
impl Decoder for NodeCodec {
    type Item = NodeRequest;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let size = {
            if src.len() < 2 {
                return Ok(None);
            }
            BigEndian::read_u16(src.as_ref()) as usize
        };

        if src.len() >= size + 2 {
            src.split_to(2);
            let buf = src.split_to(size);
            Ok(Some(json::from_slice::<NodeRequest>(&buf)?))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for NodeCodec {
    type Item = NodeResponse;
    type Error = std::io::Error;

    fn encode(&mut self, msg: NodeResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg = json::to_string(&msg).unwrap();
        let msg_ref: &[u8] = msg.as_ref();

        dst.reserve(msg_ref.len() + 2);
        dst.put_u16_be(msg_ref.len() as u16);
        dst.put(msg_ref);

        Ok(())
    }
}

pub struct ClientNodeCodec;

// Server -> Client transport
impl Decoder for ClientNodeCodec {
    type Item = NodeResponse;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let size = {
            if src.len() < 2 {
                return Ok(None);
            }
            BigEndian::read_u16(src.as_ref()) as usize
        };

        if src.len() >= size + 2 {
            src.split_to(2);
            let buf = src.split_to(size);
            Ok(Some(json::from_slice::<NodeResponse>(&buf)?))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for ClientNodeCodec {
    type Item = NodeRequest;
    type Error = std::io::Error;

    fn encode(&mut self, msg: NodeRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg = json::to_string(&msg).unwrap();
        let msg_ref: &[u8] = msg.as_ref();

        dst.reserve(msg_ref.len() + 2);
        dst.put_u16_be(msg_ref.len() as u16);
        dst.put(msg_ref);

        Ok(())
    }
}
