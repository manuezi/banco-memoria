use std::collections::{HashMap, HashSet};

#[derive(Default, Debug, Clone)]
pub struct Storage {
    data: HashMap<String, String>,
    value_to_keys: HashMap<String, HashSet<String>>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            value_to_keys: HashMap::new(),
        }
    }

    pub fn add(&mut self, key: &str, value: &str) {
        if let Some(old_value) = self.data.insert(key.to_string(), value.to_string())
            && old_value != value
        {
            let should_remove = if let Some(keys) = self.value_to_keys.get_mut(&old_value) {
                keys.remove(key);
                keys.is_empty()
            } else {
                false
            };
            if should_remove {
                self.value_to_keys.remove(&old_value);
            }
        }

        self.value_to_keys
            .entry(value.to_string())
            .or_default()
            .insert(key.to_string());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }

    pub fn get_key_by_value(&self, value: &str) -> Option<&str> {
        self.value_to_keys
            .get(value)
            .and_then(|keys| keys.iter().next().map(|s| s.as_str()))
    }
}

