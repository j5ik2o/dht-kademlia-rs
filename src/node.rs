pub const KAD_ID_LEN: usize = 160;
pub const KAD_ID_LEN_BYTES: usize = KAD_ID_LEN / 8;

pub type KadId = [u8; KAD_ID_LEN_BYTES];

pub struct Node<ID, ADDRESS> {
  kad_id: ID,
  address: ADDRESS,
}
