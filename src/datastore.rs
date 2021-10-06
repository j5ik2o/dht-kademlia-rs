use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use anyhow::Result;

pub trait DataStore {
  fn get(&self, key: &str) -> Result<Vec<u8>>;
  fn put(&mut self, key: &str, value: &[u8]) -> Result<()>;
  fn delete(&mut self, key: &str) -> Result<()>;
  fn exist(&self, key: &str) -> Result<bool>;
}

#[derive(Debug, Clone)]
pub struct InMemoryDataStore {
  store: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

unsafe impl Send for InMemoryDataStore {}
unsafe impl Sync for InMemoryDataStore {}

impl InMemoryDataStore {
  pub fn new() -> Self {
    Self {
      store: Arc::new(Mutex::new(HashMap::new())),
    }
  }
}

impl DataStore for InMemoryDataStore {
  fn get(&self, key: &str) -> Result<Vec<u8>> {
    let store = self.store.lock().unwrap();
    store.get(key).map(|e| e.clone()).ok_or(anyhow!(""))
  }

  fn put(&mut self, key: &str, value: &[u8]) -> Result<()> {
    let mut store = self.store.lock().unwrap();
    store.insert(key.to_string(), value.to_vec());
    Ok(())
  }

  fn delete(&mut self, key: &str) -> Result<()> {
    let mut store = self.store.lock().unwrap();
    store.remove(key);
    Ok(())
  }

  fn exist(&self, key: &str) -> Result<bool> {
    let store = self.store.lock().unwrap();
    Ok(store.iter().any(|(k, _v)| k == key))
  }
}
