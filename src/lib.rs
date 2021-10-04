#![feature(async_closure)]
#[cfg(test)]
extern crate env_logger as logger;
extern crate base64;

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use crate::datastore::DataStore;
use crate::node::{KadId, Node};
use crate::routing_table::RoutingTable;
use crate::transporter::{Message, Transporter, UdpTransporter};
use anyhow::Result;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;
use crate::query::KademliaMessage;

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
}

impl Kademlia {

  pub fn new(own: Node, socket_addr: SocketAddr) -> Self {
    let (msg_tx, msg_rx) = channel(128);
    Self {
      own,
      socket_addr,
      transporter: Arc::new(Mutex::new(UdpTransporter::new_with_socket_addr(socket_addr))),
      msg_tx,
      msg_rx,
    }
  }


  pub async fn bootstrap(&mut self) -> Result<()> {
    let mut lock = self.transporter.lock().await;
    lock.bind().await;
    lock.run(self.msg_tx.clone()).await;
    Ok(())
  }

  pub async fn main_routine(&mut self) {

  }

  pub async fn send_kad_msg(&self, socket_addr: SocketAddr, target: KademliaMessage) -> Result<()> {
    let data = serde_json::to_vec(&target).unwrap();
    let mut lock = self.transporter.lock().await;
    let msg = Message::new_with_ip_addr_and_port(socket_addr.ip(), socket_addr.port(), data);
    lock.send(msg).await;
    Ok(())
  }

}
