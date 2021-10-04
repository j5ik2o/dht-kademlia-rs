#![feature(async_closure)]
#[cfg(test)]
extern crate env_logger as logger;
extern crate base64;

use std::any::Any;
use std::net::{SocketAddr};
use std::sync::Arc;

use crate::node::{KadId, Node};

use crate::transporter::{Message, Transporter, UdpTransporter};
use anyhow::Result;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;
use ulid_generator_rs::{ULID, ULIDError, ULIDGenerator};
use crate::query::{KademliaMessage, Query, QueryCode};

pub mod datastore;
pub mod node;
pub mod query;
pub mod routing_table;
pub mod transporter;
pub mod utils;

pub struct Kademlia {
  own: Node,
  socket_addr: SocketAddr,
  transporter: Arc<Mutex<dyn Transporter>>,
  msg_tx: Sender<Message>,
  msg_rx: Receiver<Message>,
  // routing_table: Arc<dyn RoutingTable>,
  // data_store: Arc<dyn DataStore>,
  ulid_gen: Arc<Mutex<ULIDGenerator>>,
}

impl Kademlia {
  pub fn new(own: Node, socket_addr: SocketAddr) -> Self {
    let (msg_tx, msg_rx) = channel(128);
    Self {
      own,
      socket_addr,
      transporter: Arc::new(Mutex::new(UdpTransporter::new_with_socket_addr(
        socket_addr,
      ))),
      msg_tx,
      msg_rx,
      ulid_gen: Arc::new(Mutex::new(ULIDGenerator::new())),
    }
  }

  pub async fn bootstrap(&mut self) -> Result<()> {
    let mut lock = self.transporter.lock().await;
    lock.bind().await;
    lock.run(self.msg_tx.clone()).await;
    Ok(())
  }

  pub async fn leave(&self) {
    let mut lock = self.transporter.lock().await;
    lock.stop().await;
  }

  pub async fn main_routine(&mut self) {}

  async fn gen_ulid(&self) -> Result<ULID, ULIDError> {
    let mut ulid_gen = self.ulid_gen.lock().await;
    ulid_gen.generate()
  }

  fn is_not_same_host(&self, node: &Node) -> bool {
    self.own.socket_addr == node.socket_addr
  }

  pub async fn send_kad_msg(&self, socket_addr: SocketAddr, target: KademliaMessage) -> Result<()> {
    let mut lock = self.transporter.lock().await;
    let data = serde_json::to_vec(&target).unwrap();
    let msg = Message::new_with_socket_addr_and_data(socket_addr, data);
    lock.send(msg).await;
    Ok(())
  }

  pub async fn send_find_node_query(&self, socket_addr: SocketAddr, target: KadId) -> Result<()> {
    let msg = KademliaMessage {
      origin: self.own.clone(),
      query_sn: self.gen_ulid().await.unwrap(),
      code: QueryCode::FindNodeQuery,
      query: Query::FindNodeQuery { target },
    };
    self.send_kad_msg(socket_addr, msg).await
  }

  pub async fn send_find_node_reply(
    &self,
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
}
