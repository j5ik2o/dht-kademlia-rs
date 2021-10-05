use std::collections::HashMap;

use anyhow::anyhow;
use anyhow::Result;

pub trait DataStore {
  fn get(&self, key: &str) -> Result<&[u8]>;
  fn put(&mut self, key: &str, value: &[u8]) -> Result<()>;
  fn delete(&mut self, key: &str) -> Result<()>;
  fn exist(&self, key: &str) -> Result<bool>;
}

pub struct InMemoryDataStore {
  store: HashMap<String, Vec<u8>>,
}

impl InMemoryDataStore {
  pub fn new() -> Self {
    Self {
      store: HashMap::new(),
    }
  }
}

impl DataStore for InMemoryDataStore {
  fn get(&self, key: &str) -> Result<&[u8]> {
    self.store.get(key).map(|e| e.as_ref()).ok_or(anyhow!(""))
  }

  fn put(&mut self, key: &str, value: &[u8]) -> Result<()> {
    self.store.insert(key.to_string(), value.to_vec());
    Ok(())
  }

  fn delete(&mut self, key: &str) -> Result<()> {
    self.store.remove(key);
    Ok(())
  }

  fn exist(&self, key: &str) -> Result<bool> {
    Ok(self.store.iter().any(|(k, _v)| k == key))
  }
}
