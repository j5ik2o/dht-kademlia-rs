use crate::node::{KAD_ID_LEN, KadId, Node};

pub struct DefaultRoutingTable {
  own_id: KadId,
  table: Vec<Vec<Node>>,
}

impl DefaultRoutingTable {
  pub fn new(own_id: KadId,) -> Self {
    Self {
      own_id,
      table: Vec::with_capacity(KAD_ID_LEN)
    }
  }
}

pub trait RoutingTable {
  fn add(&mut self, node: Node) -> bool;
  fn find(&self, kid: &KadId) -> Option<&Node> ;
  fn index(&self, kid: &KadId) -> usize;
//  func (rt *routingTable) index(kid *KadID) int
}

const BUCKET_SIZE: usize = 20;

impl RoutingTable for DefaultRoutingTable {
  fn add(&mut self, node: Node) -> bool {
    let index = self.index(&node.id);
    if node.id != self.own_id && self.find(&node.id) == None && self.table[index].len() <= BUCKET_SIZE {
      self.table[index].push(node);
      true
    } else {
      false
    }
  }
  fn find(&self, kid: &KadId) -> Option<&Node> {
    todo!()
  }

  fn index(&self, kid: &KadId) -> usize {
    todo!()
  }
}

fn xor(kid1: &KadId, kid2: &KadId2) -> &KadId {
  let mut xor = KadId::default();
  for i in kid1 {

  }
}



// func xor(kid1 *KadID, kid2 *KadID) *KadID {
// xor := &KadID{}
// for i := range kid1 {
// xor[i] = kid1[i] ^ kid2[i]
// }
// return xor
// }