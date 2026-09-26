use std::{any::TypeId, collections::HashMap};

use tokio::sync::RwLock;

#[derive(Default, Debug)]
pub struct TypedMap {
    map: RwLock<HashMap<TypeId, HashMap<Value, Value>>>,
}

impl TypedMap {
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: RwLock::new(HashMap::new()),
        }
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, [`None`] is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned.
    pub async fn insert<C>(&mut self, key: Value, value: Value) -> Option<Value>
    where
        C: 'static,
    {
        let categorie = TypeId::of::<C>();

        let mut map = self.map.write().await;
        let table = map.entry(categorie).or_insert(HashMap::new());
        table.insert(key, value)
    }

    pub async fn get<C>(&self, key: Value, callback: impl FnOnce(Option<&Value>))
    where
        C: 'static,
    {
        let categorie = TypeId::of::<C>();

        let map = self.map.read().await;
        let value = map.get(&categorie).and_then(|table| table.get(&key));

        callback(value);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Value {
    String(String),
    U64(u64),
    I64(i64),
    List(Vec<Value>),
    Empty,
}

impl Value {
    #[must_use]
    pub fn as_string(&self) -> Option<&String> {
        if let Value::String(s) = self {
            Some(s)
        } else {
            None
        }
    }

    #[must_use]
    pub fn u64(&self) -> Option<u64> {
        if let Value::U64(s) = self {
            Some(*s)
        } else {
            None
        }
    }

    #[must_use]
    pub fn i64(&self) -> Option<i64> {
        if let Value::I64(s) = self {
            Some(*s)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_list(&self) -> Option<&[Value]> {
        if let Value::List(s) = self {
            Some(s.as_slice())
        } else {
            None
        }
    }
}
