use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;
use async_trait::async_trait;
use futures::TryFutureExt;

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
  socket: Arc<Mutex<UdpSocket>>,
}

#[derive(Debug, Clone)]
pub struct Message {
  ip_addr: IpAddr,
  port: u16,
  data: Vec<u8>,
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
    tokio::spawn(async move {
      loop {
        if self_cloned.terminate.load(Ordering::Relaxed) {
          break;
        }
        let mut rx = self_cloned.rx.lock().await;
        if let Some(msg) = rx.recv().await {
          log::debug!("send_to = {:?}", msg);
          let addr = SocketAddr::new(msg.ip_addr, msg.port);
          let socket = self_cloned.socket.lock().await;
          socket.send_to(&msg.data, addr).await.unwrap();
        }
      }
    });
    loop {
      if self.terminate.load(Ordering::Relaxed) {
        break;
      }
      let socket= self.socket.lock().await;
      let mut buf = [0; 1500];
      let result = socket.try_recv_from(&mut buf);
      if let Ok((_, addr)) = result {
        let msg = Message {
          ip_addr: addr.ip(),
          port: addr.port(),
          data: Vec::from(buf),
        };
        msg_tx.send(msg).await;
      }
      drop(socket);
      tokio::time::sleep(Duration::from_millis(300)).await;
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
      socket: Arc::new(Mutex::new(socket)),
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::transporter::{Message, Transporter, UdpTransporter};
  use std::net::{IpAddr, Ipv4Addr, SocketAddr};
  use std::time::Duration;
  use futures::task::SpawnExt;
  use futures::TryFutureExt;
  use tokio::net::UdpSocket;
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
    let (tx, mut rx) = channel::<Message>(128);

    tokio::spawn(async move {
      loop {
        let result = rx.try_recv();
        if let Ok(msg) = result {
          let s = String::from_utf8(msg.data.clone()).unwrap();
          log::debug!("msg = {:?}", s);
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
      }
    });

    let mut transporter: UdpTransporter = UdpTransporter::new(LISTEN_IP, SERVER_PORT).await;

    let msg_data = "abc";
    let msg = Message {
      ip_addr: LOCAL_IP,
      port: SERVER_PORT,
      data: Vec::from(msg_data),
    };
    transporter.send(msg.clone()).await;
    transporter.send(msg.clone()).await;
    transporter.send(msg.clone()).await;

    let mut transporter_cloned = transporter.clone();
    let jh = tokio::spawn(async move { transporter_cloned.run(tx).await });

    tokio::time::sleep(Duration::from_secs(3)).await;
    transporter.stop().await;

    jh.await;
  }
}
