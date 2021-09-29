use std::net::{IpAddr, UdpSocket};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use rand::{Rng, thread_rng};

#[derive(Debug)]
pub struct SendMsg {
  dest_ip: IpAddr,
  dest_port: u16,
  data: Vec<u8>,
}

impl SendMsg {
  pub fn new(dest_ip: IpAddr, dest_port: u16, data: Vec<u8>) -> Self {
    Self {
      dest_ip,
      dest_port,
      data,
    }
  }
}

#[derive(Debug)]
pub struct ReceiveMsg {
  src_ip: IpAddr,
  src_port: u16,
  data: Vec<u8>,
}

unsafe impl Sync for SendMsg {}

unsafe impl Sync for ReceiveMsg {}

#[async_trait]
trait Transporter {
  async fn run(&mut self, listen_ip: IpAddr, port: u16) -> Result<()>;
  fn stop(&mut self);
  fn send(&mut self, send_msg: SendMsg);
  fn receive_channel(&mut self) -> Arc<Receiver<ReceiveMsg>>;
}

#[derive(Clone)]
pub struct UdpTransporter {
  name: String,
  inner: Arc<Mutex<UdpTransporterInner>>,
}

struct UdpTransporterInner {
  send_tx: Sender<SendMsg>,
  receive_tx: Sender<ReceiveMsg>,
  send_rx: Receiver<SendMsg>,
  receive_rx: Arc<Receiver<ReceiveMsg>>,
  stop_flag: bool,
  socket: Option<UdpSocket>,
}

unsafe impl Send for UdpTransporter {}

impl UdpTransporter {
  pub fn new(name: &str) -> Self {
    let (send_tx, send_rx) = channel();
    let (receive_tx, receive_rx) = channel();
    Self {
      name: name.to_string(),
      inner: Arc::new(Mutex::new(UdpTransporterInner {
        send_tx,
        receive_tx,
        send_rx,
        receive_rx: Arc::new(receive_rx),
        stop_flag: false,
        socket: None,
      })),
    }
  }

  fn receiver(mut self) -> Result<()> {
    log::debug!("{}:receiver:start", self.name);
    let mut buf = [0; 1500];
    log::debug!("{}:receiver:lock", self.name);
    let inner_guard = self.inner.lock().unwrap();
    log::debug!("{}:receiver:locked", self.name);
    let udp_socket = inner_guard.socket.as_ref().unwrap();
    log::debug!("{}:receiver:udp_socket.recv_from", self.name);
    match udp_socket.recv_from(&mut buf) {
      Ok((_, src_addr)) => {
        log::debug!("{}:msg = ReceiveMsg", self.name);
        let msg = ReceiveMsg {
          src_ip: src_addr.ip(),
          src_port: src_addr.port(),
          data: Vec::from(buf),
        };
        inner_guard.receive_tx.send(msg)?;
      }
      Err(err) => {
        if inner_guard.stop_flag {
          log::debug!(
            "{}:stopped running udpTransporter @ receiver goroutine",
            self.name
          );
          return Ok(());
        }
        // log::warn!("occurred receive error: {}", err);
      }
    }
    drop(inner_guard);
    sleep(Duration::from_secs(1));
    Ok(())
  }

  fn sender(mut self) -> Result<()> {
    log::debug!("{}:sender:start", self.name);
    log::debug!("{}:sender:lock", self.name);
    let inner_guard = self.inner.lock().unwrap();
    log::debug!("{}:sender:locked", self.name);
    let send_msg_result = inner_guard.send_rx.recv_timeout(Duration::from_millis(500));
    match send_msg_result {
      Ok(send_msg) => {
        log::debug!("{}:send msg: {:?}", self.name, send_msg);
        let socket = inner_guard.socket.as_ref().unwrap();
        socket.send(&send_msg.data)?;
      }
      Err(err) => {
        log::warn!("{}:occurred send_rx error: {}", self.name, err);
      }
    }
    drop(inner_guard);
    sleep(Duration::from_secs(1));
    Ok(())
  }
}

#[async_trait]
impl Transporter for UdpTransporter {
  async fn run(&mut self, listen_ip: IpAddr, port: u16) -> Result<()> {
    log::debug!("{}:run:start", self.name);
    let duration = {
      let mut rng = thread_rng();
      rng.gen_range(0..1000)
    };
    log::debug!("{}:run:duration", self.name);
    {
      let mut inner_guard = self.inner.lock().unwrap();
      let addrs = [SocketAddr::new(listen_ip, port)];
      let s = UdpSocket::bind(&addrs[..])?;
      s.set_nonblocking(true)?;
      inner_guard.socket = Some(s);
      log::debug!("{}:run:update socket", self.name);
    }
    loop {
      log::debug!("{}:run:loop:start", self.name);
      let mut rx = self.clone();
      let mut tx = self.clone();
      rx.receiver();
      tx.sender();
      tokio::time::sleep(Duration::from_millis(3000 + duration)).await;
      log::debug!("{}:run:loop:end", self.name);
    }
  }

  fn stop(&mut self) {
    let mut inner_guard = self.inner.lock().unwrap();
    inner_guard.stop_flag = true;
  }

  fn send(&mut self, send_msg: SendMsg) {
    let inner_guard = self.inner.lock().unwrap();
    inner_guard.send_tx.send(send_msg).unwrap();
  }

  fn receive_channel(&mut self) -> Arc<Receiver<ReceiveMsg>> {
    let inner_guard = self.inner.lock().unwrap();
    inner_guard.receive_rx.clone()
  }
}

#[cfg(test)]
mod tests {
  use std::net::Ipv4Addr;
  use futures::TryFutureExt;

  use super::*;

  fn init_logger() {
    use std::env;
    env::set_var("RUST_LOG", "debug");
    // env::set_var("RUST_LOG", "trace");
    let _ = logger::try_init();
  }

  async fn server_main() {
    let listen_ip = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
    let local_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let server_port = 7001;
    let client_port = 7002;

    let mut server = UdpTransporter::new("server");
    let mut server_cloned = server.clone();
    server_cloned.run(listen_ip, server_port).await;
  }

  async fn client_main() {
    let listen_ip = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
    let local_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let server_port = 7001;
    let client_port = 7002;

    let mut client = UdpTransporter::new("client");
    let mut client_cloned = client.clone();
    client_cloned.run(listen_ip, client_port).await;
  }

  #[tokio::test]
  async fn test_tx_and_rx() {
    init_logger();
    let jh2 = tokio::spawn(server_main());
    let jh1 = tokio::spawn(client_main());

    // tokio::time::sleep(Duration::from_secs(10)).await;
    jh1.await;
    jh2.await;
  }
}
