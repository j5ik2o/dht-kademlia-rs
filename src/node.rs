use std::net::IpAddr;

pub const KAD_ID_LEN: usize = 160;
pub const KAD_ID_LEN_BYTES: usize = KAD_ID_LEN / 8;

pub type KadId = [u8; KAD_ID_LEN_BYTES];

#[derive(Clone)]
pub struct Node {
  pub(crate) id: KadId,
  pub(crate) meta: Option<NodeMeta>,
}

#[derive(Clone)]
pub struct NodeMeta {
  ip_addr: IpAddr,
  port: u16,
}

impl Node {
  pub fn new(id: KadId, meta: Option<NodeMeta>) -> Self {
    Self { id, meta }
  }
}
