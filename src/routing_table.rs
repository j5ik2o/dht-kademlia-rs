use std::cmp::Ordering;
use crate::node::{KAD_ID_LEN, KAD_ID_LEN_BYTES, KadId, Node};

pub struct DefaultRoutingTable {
  own_id: KadId,
  table: Vec<Vec<Node>>,
}

impl DefaultRoutingTable {
  pub fn new(own_id: KadId) -> Self {
    let mut table = Vec::with_capacity(KAD_ID_LEN);
    for _ in 0..KAD_ID_LEN {
      table.push(Vec::new());
    }
    Self {
      own_id,
      table
    }
  }
}

pub trait RoutingTable {
  fn own_id(&self) -> &KadId;
  fn add(&mut self, node: Node) -> bool;
  fn del(&mut self, kid: &KadId);
  fn find(&self, kid: &KadId) -> Option<&Node>;
  fn closer(&mut self, kid: &KadId) -> Vec<Node>;
  fn index(&self, kid: &KadId) -> usize;
  fn xor(&self, kid: &KadId) -> KadId;
}

const BUCKET_SIZE: usize = 20;

impl RoutingTable for DefaultRoutingTable {
  fn own_id(&self) -> &KadId {
    &self.own_id
  }

  fn add(&mut self, node: Node) -> bool {
    let index = self.index(&node.id);
    if node.id != self.own_id && self.find(&node.id).is_none() && self.table[index].len() <= BUCKET_SIZE {
      self.table[index].push(node);
      true
    } else {
      false
    }
  }

  fn del(&mut self, kid: &KadId) {
    let index = self.index(kid);
    let position_opt = self.table[index].iter().position(|e| e.id == *kid);
    if let Some(position) = position_opt {
      self.table[index].remove(position);
    }
  }

  fn find(&self, kid: &KadId) -> Option<&Node> {
    let index = self.index(kid);
    log::debug!("index = {}", index);
    self.table[index].iter().find(|e| e.id == *kid)
  }

  fn closer(&mut self, kid: &KadId) -> Vec<Node> {
    let closest_index = self.index(kid);
    let mut nodes = self.table[closest_index].clone();
    for i in 1..KAD_ID_LEN {
      let upper = closest_index + i;
      let lower = closest_index - i;
      let mut tmp = Vec::<Node>::new();
      if upper < KAD_ID_LEN {
        let iter = self.table[upper].clone();
        tmp.extend(iter);
      }
      if lower >= 0 {
        tmp.extend(self.table[lower].clone());
      }
      tmp.sort_by(|ia, jb| {
        let i_xor = xor_inner(&ia.id, kid);
        let j_xor = xor_inner(&jb.id, kid);
        let mut result = Ordering::Equal;
        for ii in 0..KAD_ID_LEN_BYTES {
          if i_xor.part(ii) == j_xor.part(ii) {
            continue;
          }
          if i_xor.part(ii) < j_xor.part(ii) {
            result = Ordering::Greater;
            break;
          } else {
            result = Ordering::Less;
            break;
          }
        }
        result
      });
      nodes.extend(tmp);
      if nodes.len() >= BUCKET_SIZE {
        return nodes.split_off(BUCKET_SIZE - 1);
      }
    }
    nodes
  }


  fn index(&self, kid: &KadId) -> usize {
    let distance = self.xor(kid);
    let mut first_bit_index = 0;
    for v in distance.get() {
      if *v == 0 {
        first_bit_index += 8;
        continue;
      }
      for i in 0..8 {
        if v & (0x80 >> i as u32) != 0 {
          break;
        }
        first_bit_index += 1;
      }
      break;
    }
    first_bit_index
  }

  fn xor(&self, kid: &KadId) -> KadId {
    xor_inner(self.own_id(), kid)
  }
}

fn xor_inner(kid1: &KadId, kid2: &KadId) -> KadId {
  let mut xor = KadId::default();
  for i in 0..KAD_ID_LEN_BYTES {
    let v = kid1.part(i) ^ kid2.part(i);
    xor.update_part(i, v);
  }
  xor
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
  fn test_xor() {
    init_logger();
    let k1_v = [0x00; KAD_ID_LEN_BYTES];
    let mut k2_v = [0xFF; KAD_ID_LEN_BYTES];
    k2_v[0] = 0xFE;
    let k1 = KadId::new(k1_v);
    let k2 = KadId::new(k2_v);

    let rt = DefaultRoutingTable::new(k1);
    let xor = rt.xor(&k2);

    let mut ans = [0xFF; KAD_ID_LEN_BYTES];
    ans[0] = 0xFE;

    assert_eq!(xor.get(), ans);
  }

  #[test]
  fn test_index() {
    init_logger();
    let own_id_v = [0x00; KAD_ID_LEN_BYTES];
    let own_id = KadId::new(own_id_v);

    let rt = DefaultRoutingTable::new(own_id);

    let mut k1_v = [0x00; KAD_ID_LEN_BYTES];
    k1_v[0] = 0xFF;
    let k1 = KadId::new(k1_v);

    let index1 = rt.index(&k1);
    assert_eq!(index1, 0);

    let mut k2_v =  [0x00; KAD_ID_LEN_BYTES];
    k2_v[k2_v.len() -1] = 0x01;
    let k2 = KadId::new(k2_v);

    let index2 = rt.index(&k2);
    assert_eq!(index2, KAD_ID_LEN-1);

    let mut k3_v = [0x00; KAD_ID_LEN_BYTES];
    k3_v[10] = 0x0F;
    let k3 = KadId::new(k3_v);

    let index3 = rt.index(&k3);
    assert_eq!(index3, 8*10+4);
  }

  struct Fields {
    own_id: KadId,
    table: Vec<Vec<Node>>
  }
  struct Args {
    node: Node,
  }
  struct Test<F> where F: FnMut(&DefaultRoutingTable) -> (bool, String) {
    name: String,
    fields: Fields,
    args: Args,
    want: bool,
    finally: Option<F>
  }

  #[test]
  fn test_add() {
    init_logger();
    let mut own_id_v = [0x00; KAD_ID_LEN_BYTES];
    own_id_v[0] = 0x01;
    let own_id = KadId::new(own_id_v);

    let mut node_id_v = [0x00; KAD_ID_LEN_BYTES];
    node_id_v[19] = 0x01;
    let node_id = KadId::new(node_id_v);

    let node = Node::new(node_id, None);
    let node_cloned = node.clone();

    let mut table = Vec::with_capacity(KAD_ID_LEN);
    for _ in 0..KAD_ID_LEN {
      table.push(Vec::new());
    }

    let tests = [
      Test{
        name: "simple add node".to_owned(),
        fields: Fields {
          own_id,
          table
        },
        args: Args {
          node
        },
        want: true,
        finally: Some(move |rt: &DefaultRoutingTable | {
          let index = rt.index(&node_cloned.id);
          if let Some(e) = rt.table[index].first() {
            (true, "".to_owned())
          } else {
            (false, "".to_owned())
          }
        })
      }
    ];

    for mut tt in tests {
      let mut rt = DefaultRoutingTable {
        own_id: tt.fields.own_id,
        table: tt.fields.table,
      };

      let got = rt.add(tt.args.node.clone());
      if got != tt.want {
        log::warn!("routingTable.add() got = {}, want {}", got, tt.want);
      }

      if let Some(f) = tt.finally {
        let (result, msg) = f(&rt);
        assert!(result);
        if !result {
          log::error!("msg = {}", msg);
        }
      }
    }
  }


}
