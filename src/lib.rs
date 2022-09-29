#![feature(async_closure)]
extern crate base64;
#[cfg(test)]
extern crate env_logger as logger;

use std::any::Any;
use std::convert::TryInto;
use std::hash::Hash;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use crate::node::{ByteArray, KadId, Node, KAD_ID_LEN_BYTES};

use crate::datastore::{DataStore, InMemoryDataStore};
use crate::query::{KademliaMessage, Query, QueryCode};
use crate::routing_table::{DefaultRoutingTable, RoutingTable};
use crate::transporter::{Message, Transporter, UdpTransporter};
use anyhow::Result;
use sha1::digest::Update;
use sha1::Digest;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;
use ulid_generator_rs::{ULIDError, ULIDGenerator, ULID};

pub mod datastore;
pub mod node;
pub mod query;
pub mod routing_table;
pub mod transporter;
pub mod utils;

#[derive(Clone)]
pub struct Kademlia {
  own: Node,
  //  socket_addr: SocketAddr,
  transporter: UdpTransporter,
  msg_tx: Sender<Message>,
  msg_rx: Arc<Mutex<Receiver<Message>>>,
  routing_table: DefaultRoutingTable,
  data_store: InMemoryDataStore,
  // find_value_callback: Option<Arc<F>>,
  ulid_gen: Arc<Mutex<ULIDGenerator>>,
}

unsafe impl Send for Kademlia {}

impl Kademlia {
  pub fn new(own: Node, data_store: InMemoryDataStore, routing_table: DefaultRoutingTable) -> Self {
    let (msg_tx, msg_rx) = channel(128);
    let tx = UdpTransporter::new_with_ip_addr_and_port(own.socket_addr.ip().clone(), own.socket_addr.port());
    Self {
      own: own.clone(),
      // socket_addr: own.socket_addr.clone(),
      transporter: tx,
      msg_tx,
      msg_rx: Arc::new(Mutex::new(msg_rx)),
      data_store,
      routing_table,
      ulid_gen: Arc::new(Mutex::new(ULIDGenerator::new())),
    }
  }

  pub async fn leave(&mut self) {
    self.transporter.stop().await;
  }

  pub async fn store(&mut self, key: &str, value: &[u8]) {
    self.data_store.put(key, value).unwrap();

    let key_kid: KadId = key.to_owned().try_into().unwrap();
    let closest = self.routing_table.closer(&key_kid);
    for node in closest.iter() {
      self.send_store_query(node, key).await.unwrap();
    }
  }

  pub async fn find_value(&mut self, key: &str) {
    let b = { self.data_store.exist(key).unwrap() };
    if b {
      // if let Some(f) = self.find_value_callback.clone() {
      //   let b = data_store.get(key).unwrap();
      //   log::info!("b = {}", b)
      //   // (*f)(key, Vec::from(b))
      // }
    } else {
      let kid: KadId = key.to_owned().try_into().unwrap();
      let closest = self.routing_table.closer(&kid);
      if !closest.is_empty() {
        self.send_find_value_query(&closest[0], key).await.unwrap();
      }
    }
  }

  pub async fn bootstrap(&mut self, seed_node_socket_addr_opt: Option<SocketAddr>) -> Result<()> {
    // if let Some(seed_node_socket_addr) = seed_node_socket_addr_opt {
    //   use resolve::resolve_host;
    //   let entry_node_ip_addr: IpAddr = resolve_host(seed_node_addr)
    //     .unwrap()
    //     .into_iter()
    //     .next()
    //     .unwrap();
    // }

    let jh1 = {
      let mut transporter_cloned = self.transporter.clone();
      let msg_tx_cloned = self.msg_tx.clone();
      tokio::spawn(async move {
        //        let mut lock = transporter_cloned.lock().await;
        log::debug!("bind");
        transporter_cloned.bind().await;
        log::debug!("run");
        transporter_cloned.run(msg_tx_cloned).await
      })
    };

    let mut self_cloned = self.clone();
    let jh2 = tokio::spawn(async move {
      self_cloned.main_routine(seed_node_socket_addr_opt).await;
    });
    tokio::join!(jh1, jh2);
    Ok(())
  }

  pub async fn main_routine(&mut self, seed_node_socket_addr_opt: Option<SocketAddr>) {
    if let Some(seed_node_socket_addr) = seed_node_socket_addr_opt {
      log::debug!("send_find_node_query");
      self
        .send_find_node_query(seed_node_socket_addr, self.own.id.clone())
        .await
        .unwrap();
      log::debug!("done:send_find_node_query");
    }
    loop {
      let msg = {
        let mut msg_rx = self.msg_rx.lock().await;
        msg_rx.recv().await.unwrap()
      };
      let s = String::from_utf8(msg.data.clone());
      let km: KademliaMessage = serde_json::from_slice(&msg.data).unwrap();
      match km.query {
        Query::PingQuery { target } => {
          log::debug!("Query::PingQuery:target = {:?}, self = {:?}", target, self.own.id);
          log::debug!("Query::send_ping_reply");
          self
            .send_ping_reply(msg.socket_addr, km.query_sn, self.own.id.clone())
            .await
            .unwrap();
          log::debug!("done:Query::send_ping_reply");
          if !self.routing_table.find(&target).is_some() {
            self.routing_table.add(km.origin);
          }
          log::debug!("done:Query::PingQuery:target = {:?}", target);
        }
        Query::FindNodeQuery { target } => {
          log::debug!("Query::FindNodeQuery:target = {:?}", target);
          let closest = self.routing_table.closer(&target);
          self
            .send_find_node_reply(msg.socket_addr, km.query_sn, closest)
            .await
            .unwrap();
          self.routing_table.add(km.origin);
          log::debug!("done:Query::FindNodeQuery: rt = {:?}", self.routing_table);
        }
        Query::FindNodeReply { closest } => {
          log::debug!("Query::FindNodeReply:closest = {:?}", closest);
          log::debug!("self.routing_table.add(km.origin)");
          self.routing_table.add(km.origin);
          log::debug!("done:self.routing_table.add(km.origin)");
          for node in closest.iter() {
            log::debug!("if self.is_not_same_host(node) && self.routing_table.find(&node.id).is_none()");
            if self.is_not_same_host(node) && self.routing_table.find(&node.id).is_none() {
              log::debug!("send_find_node_query");
              self
                .send_find_node_query(node.socket_addr, self.own.id.clone())
                .await
                .unwrap();
              log::debug!("done:send_find_node_query");
            }
          }
          log::debug!("done:Query::FindNodeReply: rt = {:?}", self.routing_table);
        }
        Query::StoreQuery { key, data } => {
          log::debug!("Query::StoreQuery:key = {:?}, data = {:?}", key, data);
          self.data_store.put(&key, &data).unwrap();
        }
        Query::FindValueQuery { key } => {
          log::debug!("Query::FindValueQuery:key = {:?}", key);
          let (has_value, value) = {
            let has_value = self.data_store.exist(&key).unwrap();
            let data = self.data_store.get(&key).unwrap();
            (has_value, Vec::from(data))
          };
          let kid: KadId = key.clone().try_into().unwrap();
          let closest = self.routing_table.closer(&kid);
          self
            .send_find_value_reply(msg.socket_addr, km.query_sn, &key, has_value, value, closest)
            .await
            .unwrap();
        }
        Query::FindValueReply {
          key,
          has_value,
          value,
          closest,
        } => {
          log::debug!("Query::FindValueReply:key = {:?}", key);
          if has_value {
            // callback
          } else {
            if !closest.is_empty() {
              let nex_inquiry_node = &closest[0];
              self.send_find_value_query(nex_inquiry_node, &key).await.unwrap();
            }
          }
        }
        msg => {
          log::debug!("otherwise: msg = {:?}", msg)
        }
      }
    }
  }

  async fn gen_ulid(&self) -> Result<ULID, ULIDError> {
    let mut ulid_gen = self.ulid_gen.lock().await;
    ulid_gen.generate()
  }

  fn is_not_same_host(&self, node: &Node) -> bool {
    self.own.socket_addr == node.socket_addr
  }

  pub async fn send_kad_msg(&mut self, socket_addr: SocketAddr, target: KademliaMessage) -> Result<()> {
    let data = serde_json::to_vec(&target).unwrap();
    let msg = Message::new_with_socket_addr_and_data(socket_addr, data);
    self.transporter.send(msg).await;
    Ok(())
  }

  pub async fn send_find_node_query(&mut self, socket_addr: SocketAddr, target: KadId) -> Result<()> {
    let msg = KademliaMessage {
      origin: self.own.clone(),
      query_sn: self.gen_ulid().await.unwrap(),
      code: QueryCode::FindNodeQuery,
      query: Query::FindNodeQuery { target },
    };
    self.send_kad_msg(socket_addr, msg).await
  }

  pub async fn send_find_node_reply(
    &mut self,
    socket_addr: SocketAddr,
    query_sn: ULID,
    closest: Vec<Node>,
  ) -> Result<()> {
    let msg = KademliaMessage {
      origin: self.own.clone(),
      query_sn,
      code: QueryCode::FindNodeReply,
      query: Query::FindNodeReply { closest },
    };
    self.send_kad_msg(socket_addr, msg).await
  }

  pub async fn send_store_query(&mut self, node: &Node, key: &str) -> Result<()> {
    let data = {
      let data = self.data_store.get(key).unwrap();
      Vec::from(data)
    };

    let msg = KademliaMessage {
      origin: self.own.clone(),
      query_sn: self.gen_ulid().await.unwrap(),
      code: QueryCode::StoreQuery,
      query: Query::StoreQuery {
        key: key.to_owned(),
        data,
      },
    };
    self.send_kad_msg(node.socket_addr, msg).await
  }

  pub async fn send_find_value_query(&mut self, node: &Node, key: &str) -> Result<()> {
    let msg = KademliaMessage {
      origin: self.own.clone(),
      query_sn: self.gen_ulid().await.unwrap(),
      code: QueryCode::FindValueQuery,
      query: Query::FindValueQuery { key: key.to_string() },
    };
    self.send_kad_msg(node.socket_addr, msg).await
  }

  pub async fn send_find_value_reply(
    &mut self,
    socket_addr: SocketAddr,
    query_sn: ULID,
    key: &str,
    has_value: bool,
    value: Vec<u8>,
    closest: Vec<Node>,
  ) -> Result<()> {
    let msg = KademliaMessage {
      origin: self.own.clone(),
      query_sn,
      code: QueryCode::FindValueReply,
      query: Query::FindValueReply {
        key: key.to_owned(),
        has_value,
        value,
        closest,
      },
    };
    self.send_kad_msg(socket_addr, msg).await
  }

  pub async fn send_ping_query(&mut self, socket_addr: SocketAddr, target: KadId) -> Result<()> {
    let msg = KademliaMessage {
      origin: self.own.clone(),
      query_sn: self.gen_ulid().await.unwrap(),
      code: QueryCode::PingQuery,
      query: Query::PingQuery { target },
    };
    self.send_kad_msg(socket_addr, msg).await
  }

  pub async fn send_ping_reply(&mut self, socket_addr: SocketAddr, query_sn: ULID, target: KadId) -> Result<()> {
    let msg = KademliaMessage {
      origin: self.own.clone(),
      query_sn,
      code: QueryCode::PingReply,
      query: Query::PingReply { target },
    };
    self.send_kad_msg(socket_addr, msg).await
  }
}

#[cfg(test)]
mod tests {
  use crate::datastore::InMemoryDataStore;
  use crate::node::{KadId, Node, KAD_ID_LEN_BYTES};
  use crate::routing_table::DefaultRoutingTable;
  use crate::Kademlia;
  use std::convert::TryInto;
  use std::net::SocketAddr;
  use tokio::time::Duration;

  #[ctor::ctor]
  fn init_logger() {
    use std::env;
    env::set_var("RUST_LOG", "debug");
    // env::set_var("RUST_LOG", "trace");
    let _ = logger::try_init();
  }

  #[tokio::test]
  async fn test() {
    let addr = resolve::resolve_host("localhost").unwrap().next().unwrap();
    let listen_socket_addr1 = SocketAddr::new(addr, 7005);
    let listen_socket_addr2 = SocketAddr::new(addr, 7006);
    let seed_node_socket_addr = "127.0.0.1:7005".parse::<SocketAddr>().unwrap();

    let mut own_id_v1 = [0x00; KAD_ID_LEN_BYTES];
    own_id_v1[0] = 0x01;
    let node1 = Node::new(KadId::new(own_id_v1), listen_socket_addr1);

    let mut own_id_v2 = [0x00; KAD_ID_LEN_BYTES];
    own_id_v2[0] = 0x02;
    let node2 = Node::new(KadId::new(own_id_v2), listen_socket_addr2);

    log::debug!("node1 = {:?}", node1);
    log::debug!("node2 = {:?}", node2);

    let mut kad1 = Kademlia::new(
      node1.clone(),
      InMemoryDataStore::new(),
      DefaultRoutingTable::new(node1.id.clone()),
    );
    let mut kad1_cloned = kad1.clone();
    let jh1 = tokio::spawn(async move {
      kad1_cloned.bootstrap(None).await;
    });

    let mut kad2 = Kademlia::new(
      node2.clone(),
      InMemoryDataStore::new(),
      DefaultRoutingTable::new(node2.id.clone()),
    );
    let mut kad2_cloned = kad2.clone();
    let jh2 = tokio::spawn(async move {
      kad2_cloned.bootstrap(Some(seed_node_socket_addr)).await;
    });

    tokio::time::sleep(Duration::from_secs(5)).await;

    kad2
      .send_ping_query(seed_node_socket_addr, kad2.own.id.clone())
      .await
      .unwrap();

    tokio::time::sleep(Duration::from_secs(5)).await;

    kad1.leave().await;
    kad2.leave().await;
  }
}
