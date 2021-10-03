use std::convert::{TryFrom, TryInto};
use std::fmt::Formatter;
use std::net::IpAddr;
use std::str::FromStr;
use rand::{RngCore, thread_rng};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::de::{Error, Visitor};
use thiserror::Error;

pub const KAD_ID_LEN: usize = 160;
pub const KAD_ID_LEN_BYTES: usize = KAD_ID_LEN / 8;

pub type ByteArray = [u8; KAD_ID_LEN_BYTES];

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct KadId(ByteArray);

#[derive(Debug, Error, Clone, PartialEq)]
pub enum KadIdError {
  #[error("Invalid byte array")]
  InvalidByteArrayError,
}

impl KadId {
  pub fn generate() -> KadId {
    let mut rng = thread_rng();
    let mut values = [0u8; KAD_ID_LEN_BYTES];
    rng.fill_bytes(&mut values);
    KadId::new(values)
  }
  pub fn parse_from_base64str(s: &str) -> Result<KadId, KadIdError> {
    let br = base64::decode(s);
    match br {
      Err(e) => Err(KadIdError::InvalidByteArrayError),
      Ok(b) => {
        let ba = b.try_into();
        match ba {
          Ok(b) => Ok(KadId::new(b)),
          Err(_) => Err(KadIdError::InvalidByteArrayError),
        }
      }
    }
  }
  pub fn new(v: ByteArray) -> Self {
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
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(&self.to_base64())
  }
}

impl<'de> Deserialize<'de> for KadId {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let deserialized_str = String::deserialize(deserializer)?;
    let s = KadId::parse_from_base64str(&deserialized_str).map_err(serde::de::Error::custom);
    log::debug!("de:s = {:?}", s);
    s
  }
}

impl Default for KadId {
  fn default() -> Self {
    Self([0; KAD_ID_LEN_BYTES])
  }
}

impl From<String> for KadId {
  fn from(v: String) -> Self {
    let s = base64::decode(v).unwrap().try_into().unwrap();
    Self::new(s)
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
  pub(crate) id: KadId,
  pub(crate) meta: Option<NodeMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
  fn test_kid() {
    init_logger();
    let mut own_id = [0x00; KAD_ID_LEN_BYTES];
    own_id[0] = 0x01;
    let kid = KadId::new(own_id);
    let s = serde_json::to_string(&kid).unwrap();
    log::debug!("s = {}", s);
    let kid2: KadId = serde_json::from_str(&s).unwrap();
    log::debug!("kid = {:?}", kid2);
    assert_eq!(kid, kid2)
  }

  #[test]
  fn test_node() {
    init_logger();
    let kid = KadId::generate();
    let node1 = Node::new(kid, None);
    let s = serde_json::to_string(&node1).unwrap();
    log::debug!("s = {}", s);
    let node2: Node = serde_json::from_str(&s).unwrap();
    log::debug!("n = {:?}", node2);
  }
}
