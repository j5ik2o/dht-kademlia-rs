use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::{Mutex};
use async_trait::async_trait;

#[async_trait]
pub trait Transporter {
  async fn stop(&mut self);
  async fn send(&mut self, msg: Message);
  async fn run(&mut self, mut msg_tx: Sender<Message>);
}

#[derive(Clone)]
pub struct UdpTransporter {
  tx: Sender<Message>,
  rx: Arc<Mutex<Receiver<Message>>>,
  terminate: Arc<AtomicBool>,
  socket: Arc<UdpSocket>,
}

#[derive(Debug, Clone)]
pub struct Message {
  ip_addr: IpAddr,
  port: u16,
  data: Vec<u8>,
}

impl Message {
  pub fn new(  ip_addr: IpAddr,
               port: u16,
               data: Vec<u8>) -> Self {
    Self {
      ip_addr,
      port,
      data
    }
  }
}

impl UdpTransporter {
  pub async fn new(ip_addr: IpAddr, port: u16) -> UdpTransporter {
    let addresses = [SocketAddr::new(ip_addr, port)];
    let socket = UdpSocket::bind(&addresses[..]).await.unwrap();
    let (tx, rx) = channel(128);
    Self {
      tx,
      rx: Arc::new(Mutex::new(rx)),
      terminate: Arc::new(AtomicBool::new(false)),
      socket: Arc::new(socket),
    }
  }

  async fn send_message_to_upstream(&self) {
    loop {
      if self.terminate.load(Ordering::Relaxed) {
        break;
      }
      let mut rx = self.rx.lock().await;
      if let Some(msg) = rx.recv().await {
        // log::debug!("send_to = {:?}", msg);
        let addr = SocketAddr::new(msg.ip_addr, msg.port);
        let _ = self.socket.send_to(&msg.data, addr).await.unwrap();
      }
    }
  }

  async fn receive_message_from_downstream(&mut self, msg_tx: Sender<Message>) {
    let mut buf = [0; 1500];
    let result = self.socket.try_recv_from(&mut buf);
    if let Ok((_, addr)) = result {
      Self::send_message_to_tx(msg_tx, Vec::from(buf), addr).await;
    }
  }

  async fn send_message_to_tx(msg_tx: Sender<Message>, buf: Vec<u8>, addr: SocketAddr) {
    let msg = Message::new(
      addr.ip(),
      addr.port(),
      buf,
    );
    let _ = msg_tx.send(msg).await;
  }
}

#[async_trait]
impl Transporter for UdpTransporter {
  async fn stop(&mut self) {
    self.terminate.store(true, Ordering::Relaxed);
  }

  async fn send(&mut self, msg: Message) {
    self.tx.send(msg).await;
  }

  async fn run(&mut self, msg_tx: Sender<Message>) {
    let self_cloned = self.clone();
    tokio::spawn( async move { self_cloned.send_message_to_upstream().await; });
    loop {
      if self.terminate.load(Ordering::Relaxed) {
        break;
      }
      self.receive_message_from_downstream(msg_tx.clone()).await;
      tokio::time::sleep(Duration::from_millis(300)).await;
    }
  }
}



#[cfg(test)]
mod tests {
  use crate::transporter::{Message, Transporter, UdpTransporter};
  use std::net::{IpAddr, Ipv4Addr};
  use std::time::Duration;

  use tokio::sync::mpsc::channel;

  const LISTEN_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
  const LOCAL_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
  const SERVER_PORT: u16 = 7001;
  const CLIENT_PORT: u16 = 7002;

  fn init_logger() {
    use std::env;
    env::set_var("RUST_LOG", "debug");
    // env::set_var("RUST_LOG", "trace");
    let _ = logger::try_init();
  }

  #[tokio::test]
  async fn test_transport() {
    init_logger();
    let (server_tx, mut server_rx) = channel::<Message>(128);
    let (client_tx, mut client_rx) = channel::<Message>(128);

    let mut server: UdpTransporter = UdpTransporter::new(LISTEN_IP, SERVER_PORT).await;
    let mut client: UdpTransporter = UdpTransporter::new(LISTEN_IP, CLIENT_PORT).await;

    let mut sever_clone = server.clone();
    tokio::spawn(async move {
      loop {
        let result = server_rx.try_recv();
        if let Ok(msg) = result {
          let s = String::from_utf8(msg.data.clone()).unwrap();
          log::debug!("server:receive: msg = {:?}", s);
          sever_clone.send(msg).await;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
      }
    });

    tokio::spawn(async move {
      loop {
        let result = client_rx.try_recv();
        if let Ok(msg) = result {
          let s = String::from_utf8(msg.data.clone()).unwrap();
          log::debug!("client:receive: msg = {:?}", s);
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
      }
    });

    let mut server_cloned = server.clone();
    let jh1 = tokio::spawn(async move { server_cloned.run(server_tx).await });

    let mut client_cloned = client.clone();
    let jh2 = tokio::spawn(async move { client_cloned.run(client_tx).await });

    let mut client_cloned = client.clone();
    tokio::spawn(async move {
      let msg_data = "abc";
      let msg = Message::new(
        LOCAL_IP,
        SERVER_PORT,
        Vec::from(msg_data),
      );
      client_cloned.send(msg.clone()).await;
      client_cloned.send(msg.clone()).await;
      client_cloned.send(msg.clone()).await;
    }).await;

    tokio::time::sleep(Duration::from_secs(3)).await;

    server.stop().await;
    client.stop().await;

    jh1.await;
    jh2.await;
  }
}
