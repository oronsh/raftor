use byteorder::{BigEndian, ByteOrder};
use bytes::{BufMut, BytesMut};
use tokio::codec::{Decoder, Encoder};
use serde::{Serialize, Deserialize};
use serde_json as json;
use actix_raft::{NodeId};

#[derive(Serialize, Deserialize, Debug)]
pub enum NodeRequest {
    Ping,
    Join(NodeId),
    /// Message(msg_id, type_id, payload)
    Message(u64, MsgTypes, String),
}
#[derive(Serialize, Deserialize, Debug)]
pub enum NodeResponse {
    Ping,
    Joined,
    /// Result(msg_id, payload)
    Result(u64, String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MsgTypes {
    AppendEntriesRequest,
    VoteRequest,
    InstallSnapshotRequest,
    ClientPayload,
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
