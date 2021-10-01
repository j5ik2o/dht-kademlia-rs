use crate::node::KadId;
use rand::{thread_rng, RngCore};

#[cfg(target_os = "windows")]
fn get_ip_list() -> Vec<String> {
  use ipconfig;
  let mut ips: Vec<String> = Vec::new();
  match ipconfig::get_adapters() {
    Ok(adapters) => {
      for adapter in adapters {
        if adapter.oper_status() == ipconfig::OperStatus::IfOperStatusUp {
          for address in adapter.ip_addresses() {
            if !address.is_loopback() && address.is_ipv4() {
              ips.push(address.to_string());
            }
          }
        }
      }
    }
    _ => {}
  }
  ips
}

#[cfg(not(target_os = "windows"))]
fn get_ip_list() -> Vec<String> {
  use pnet::datalink;
  let mut ips: Vec<String> = Vec::new();
  for interface in datalink::interfaces() {
    // 空ではなく、動作中である。
    if !interface.ips.is_empty() && interface.is_up() {
      for ip_net in interface.ips {
        // ループバックでなく、ipv4である
        if ip_net.is_ipv4() && !ip_net.ip().is_loopback() {
          ips.push(ip_net.ip().to_string());
        }
      }
    }
  }
  ips
}

fn generate_random_kad_id() -> KadId {
  let mut rng = thread_rng();
  let mut values = KadId::default();
  rng.fill_bytes(&mut values);
  values
}

#[cfg(test)]
mod tests {
  use crate::utils::get_ip_list;

  fn init_logger() {
    use std::env;
    env::set_var("RUST_LOG", "debug");
    // env::set_var("RUST_LOG", "trace");
    let _ = logger::try_init();
  }

  #[test]
  fn test_get_ip_list() {
    init_logger();
    let ip_list = get_ip_list();
    let first = ip_list.first().unwrap();
    log::debug!("first = {}", first);
  }
}
