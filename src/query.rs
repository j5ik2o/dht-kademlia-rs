use serde::{Deserialize, Serialize};
use crate::node::{KadId, Node};

#[derive(Serialize, Deserialize)]
pub struct KademliaMessage {
  origin: Node,
  query_sn: i64,
  type_id: i32,
  query: Query,
}

#[derive(Serialize, Deserialize)]
pub enum Query {
  PingQuery {
    target: KadId,
  },
  StoreQuery {
    key: String,
    data: Vec<u8>,
  },
  FindNodeQuery {
    target: KadId,
  },
  FindValueQuery {
    target: KadId,
  },
  PingReply {
    target: KadId,
  },
  StoreReply {
    success: bool,
  },
  FindNodeReply {
    key: String,
    value: Vec<u8>,
    closest: Vec<Node>,
  },
  FindValueReply {
    key: String,
  },
}

impl Query {
  pub fn serialize(msg: &KademliaMessage) -> serde_json::Result<String> {
    serde_json::to_string(msg)
  }

  pub fn deserialize(msg: &str) -> serde_json::Result<KademliaMessage> {
    serde_json::from_str(msg)
  }
}

#[cfg(test)]
mod tests {

  #[test]
  fn test() {}
}
