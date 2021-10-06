use serde::{Deserialize, Serialize};
use ulid_generator_rs::ULID;
use crate::node::{KadId, Node};

#[derive(Debug, Serialize, Deserialize)]
pub struct KademliaMessage {
  pub origin: Node,
  pub query_sn: ULID,
  pub code: QueryCode,
  pub query: Query,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum QueryCode {
  PingQuery,
  StoreQuery,
  FindNodeQuery,
  FindValueQuery,
  PingReply,
  StoreReply,
  FindNodeReply,
  FindValueReply,
}

#[derive(Debug, Serialize, Deserialize)]
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
    key: String,
  },
  PingReply {
    target: KadId,
  },
  StoreReply {
    success: bool,
  },
  FindNodeReply {
    closest: Vec<Node>,
  },
  FindValueReply {
    key: String,
    has_value: bool,
    value: Vec<u8>,
    closest: Vec<Node>,
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
  use ulid_generator_rs::ULIDGenerator;
  use crate::node::{KadId, Node};
  use crate::query::{KademliaMessage, Query, QueryCode};

  fn init_logger() {
    use std::env;
    env::set_var("RUST_LOG", "debug");
    // env::set_var("RUST_LOG", "trace");
    let _ = logger::try_init();
  }

  #[test]
  fn test() {
    init_logger();
    let mut gen = ULIDGenerator::new();
    let id = gen.generate().unwrap();
    let msg = KademliaMessage {
      origin: Node::default(),
      query_sn: id,
      code: QueryCode::FindNodeQuery,
      query: Query::FindNodeQuery {
        target: KadId::default(),
      },
    };
    let s = serde_json::to_vec(&msg).unwrap();
    let d: KademliaMessage = serde_json::from_slice(&s).unwrap();
    log::debug!("s = {:?}", d);
  }
}
