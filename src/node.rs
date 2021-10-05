use std::convert::{Infallible, TryFrom, TryInto};
use std::fmt::Formatter;

use std::net::{IpAddr, SocketAddr};

use rand::{RngCore, thread_rng};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use sha1::Digest;
use sha1::digest::Update;

use thiserror::Error;

pub const KAD_ID_LEN: usize = 160;
pub const KAD_ID_LEN_BYTES: usize = KAD_ID_LEN / 8;

pub type ByteArray = [u8; KAD_ID_LEN_BYTES];

#[derive(Clone, PartialOrd, PartialEq)]
pub struct KadId(ByteArray);

#[derive(Debug, Error, Clone, PartialEq)]
pub enum KadIdError {
  #[error("Invalid byte array")]
  InvalidByteArrayError,
}

impl std::fmt::Debug for KadId {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.to_hex())
  }
}

impl std::fmt::Display for KadId {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.to_hex())
  }
}

impl TryFrom<String> for KadId {
  type Error = KadIdError;

  fn try_from(value: String) -> Result<Self, Self::Error> {
    use sha1::Sha1;
    let mut hasher = Sha1::new();
    Update::update(&mut hasher, value.as_bytes());
    let hashed_value_output = hasher.finalize();
    let mut value = [0u8; KAD_ID_LEN_BYTES];
    let hashed_value_result = hashed_value_output[..].try_into();
    match hashed_value_result {
      Ok(hashed_value) => {
        std::mem::replace(&mut value, hashed_value);
        Ok(KadId::new(value))
      }
      Err(e) => Err(KadIdError::InvalidByteArrayError),
    }
  }
}

impl KadId {
  pub fn generate() -> KadId {
    let mut rng = thread_rng();
    let mut values = [0u8; KAD_ID_LEN_BYTES];
    rng.fill_bytes(&mut values);
    KadId::new(values)
  }

  pub fn parse_from_hexstr(s: &str) -> Result<KadId, KadIdError> {
    let br = hex::decode(s);
    match br {
      Err(_e) => Err(KadIdError::InvalidByteArrayError),
      Ok(b) => {
        let ba = b.try_into();
        match ba {
          Ok(b) => Ok(KadId::new(b)),
          Err(_) => Err(KadIdError::InvalidByteArrayError),
        }
      }
    }
  }

  pub fn parse_from_base64str(s: &str) -> Result<KadId, KadIdError> {
    let br = base64::decode(s);
    match br {
      Err(_e) => Err(KadIdError::InvalidByteArrayError),
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

  pub fn to_hex(&self) -> String {
    hex::encode(self.0)
  }
}

impl Serialize for KadId {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(&self.to_hex())
  }
}

impl<'de> Deserialize<'de> for KadId {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let deserialized_str = String::deserialize(deserializer)?;
    let s = KadId::parse_from_hexstr(&deserialized_str).map_err(serde::de::Error::custom);
    log::debug!("de:s = {:?}", s);
    s
  }
}

impl Default for KadId {
  fn default() -> Self {
    Self([0; KAD_ID_LEN_BYTES])
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
  pub(crate) id: KadId,
  pub(crate) socket_addr: SocketAddr,
}

impl Default for Node {
  fn default() -> Self {
    Self {
      id: KadId::default(),
      socket_addr: SocketAddr::new("127.0.0.1".parse::<IpAddr>().unwrap(), 3330),
    }
  }
}

impl Node {
  pub fn new(id: KadId, socket_addr: SocketAddr) -> Self {
    Self { id, socket_addr }
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
    let node1 = Node::new(kid, "127.0.0.1:3330".parse::<SocketAddr>().unwrap());
    let s = serde_json::to_string(&node1).unwrap();
    log::debug!("s = {}", s);
    let node2: Node = serde_json::from_str(&s).unwrap();
    log::debug!("n = {:?}", node2);
  }
}
