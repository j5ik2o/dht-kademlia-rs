use std::convert::TryInto;
use std::fmt::Formatter;
use std::net::IpAddr;
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::de::Visitor;

pub const KAD_ID_LEN: usize = 160;
pub const KAD_ID_LEN_BYTES: usize = KAD_ID_LEN / 8;

#[derive(Clone, PartialOrd, PartialEq)]
pub struct KadId([u8; KAD_ID_LEN_BYTES]);

impl KadId {
  pub fn new(v: [u8; KAD_ID_LEN_BYTES]) -> Self {
    Self(v)
  }
  pub fn update_part(&mut self, pos: usize, v: u8) {
    self.0[pos] = v;
  }
  pub fn get(&self) -> &[u8] {
    &self.0
  }
  pub fn get_mut(&mut self) -> &mut [u8] {
    &mut self.0
  }
  pub fn part(&self, pos: usize) -> u8 {
    self.0[pos]
  }
  pub fn to_base64(&self) -> String {
    base64::encode(self.0)
  }
}

impl Serialize for KadId {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    serializer.serialize_str(&self.to_base64())
  }
}

struct KadIdVisitor;

impl<'de> Visitor<'de> for KadIdVisitor {
  type Value = KadId;

  fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
    formatter.write_str("an integer between -2^31 and 2^31")
  }

  fn visit_string<E>(self, v: String) -> Result<Self::Value, E> where E: serde::de::Error {
    Ok(v.into())
  }
}

impl<'de> Deserialize<'de> for KadId {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
    deserializer.deserialize_string(KadIdVisitor)
  }
}

impl Default for KadId {
  fn default() -> Self {
    Self([0; KAD_ID_LEN_BYTES])
  }
}
impl From<[u8; KAD_ID_LEN_BYTES]> for KadId {
  fn from(v: [u8; KAD_ID_LEN_BYTES]) -> Self {
    Self(v)
  }
}
impl From<String> for KadId {
  fn from(v: String) -> Self {
    let s = base64::decode(v).unwrap().try_into().unwrap();
    Self::new(s)
  }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Node {
  pub(crate) id: KadId,
  pub(crate) meta: Option<NodeMeta>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct NodeMeta {
  ip_addr: IpAddr,
  port: u16,
}

impl Node {
  pub fn new(id: KadId, meta: Option<NodeMeta>) -> Self {
    Self { id, meta }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn init_logger() {
    use std::env;
    env::set_var("RUST_LOG", "debug");
    // env::set_var("RUST_LOG", "trace");
    let _ = logger::try_init();
  }
  #[test]
  fn test() {
    init_logger();
    let mut own_id = [0x00; KAD_ID_LEN_BYTES];
    own_id[0] = 0x01;
    let node = Node::new(own_id.into(), None);
    let s = serde_json::to_string(&node).unwrap();
    log::debug!("s = {}", s);
  }

}
