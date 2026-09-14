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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get() {
        let mut storage = Storage::new();

        storage.add("user_1", "Alice");
        assert_eq!(storage.get("user_1"), Some("Alice"));
        assert_eq!(storage.get("missing_key"), None);
        assert_eq!(storage.get_key_by_value("Alice"), Some("user_1"));
        assert_eq!(storage.get_key_by_value("Bob"), None);
    }

    #[test]
    fn test_overwrite() {
        let mut storage = Storage::new();

        storage.add("key", "initial");
        assert_eq!(storage.get_key_by_value("initial"), Some("key"));

        storage.add("key", "updated");
        assert_eq!(storage.get("key"), Some("updated"));
        assert_eq!(storage.get_key_by_value("initial"), None);
        assert_eq!(storage.get_key_by_value("updated"), Some("key"));
    }

    #[test]
    fn test_multiple_keys_same_value() {
        let mut storage = Storage::new();

        storage.add("user_1", "Alice");
        storage.add("user_2", "Alice");

        assert_eq!(storage.get("user_1"), Some("Alice"));
        assert_eq!(storage.get("user_2"), Some("Alice"));

        let key = storage.get_key_by_value("Alice");
        assert!(key == Some("user_1") || key == Some("user_2"));

        // Update user_1, user_2 still has Alice
        storage.add("user_1", "Bob");
        assert_eq!(storage.get_key_by_value("Alice"), Some("user_2"));
        assert_eq!(storage.get_key_by_value("Bob"), Some("user_1"));

        // Update user_2 as well, Alice no longer exists
        storage.add("user_2", "Charlie");
        assert_eq!(storage.get_key_by_value("Alice"), None);
    }
}
