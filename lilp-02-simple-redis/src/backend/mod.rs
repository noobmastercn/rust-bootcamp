use crate::{BulkString, RespFrame};
use dashmap::{DashMap, DashSet};
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Backend(Arc<BackendInner>);

#[derive(Debug)]
pub struct BackendInner {
    pub(crate) map: DashMap<String, RespFrame>,
    pub(crate) hmap: DashMap<String, DashMap<String, RespFrame>>,
    pub(crate) set: DashMap<String, DashSet<BulkString>>,
}

impl Deref for Backend {
    type Target = BackendInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for Backend {
    fn default() -> Self {
        Self(Arc::new(BackendInner::default()))
    }
}

impl Default for BackendInner {
    fn default() -> Self {
        Self {
            map: DashMap::new(),
            hmap: DashMap::new(),
            set: DashMap::new(),
        }
    }
}

impl Backend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<RespFrame> {
        self.map.get(key).map(|v| v.value().clone())
    }

    pub fn set(&self, key: String, value: RespFrame) {
        self.map.insert(key, value);
    }

    pub fn hget(&self, key: &str, field: &str) -> Option<RespFrame> {
        self.hmap
            .get(key)
            .and_then(|v| v.get(field).map(|v| v.value().clone()))
    }

    pub fn hset(&self, key: String, field: String, value: RespFrame) {
        let hmap = self.hmap.entry(key).or_default();
        hmap.insert(field, value);
    }

    pub fn hgetall(&self, key: &str) -> Option<DashMap<String, RespFrame>> {
        self.hmap.get(key).map(|v| v.clone())
    }

    pub fn hmget(&self, key: &str, fields: &[String]) -> Option<Vec<Option<RespFrame>>> {
        self.hmap.get(key).map(|v| {
            fields
                .iter()
                .map(|f| v.get(f).map(|v| v.value().clone()))
                .collect()
        })
    }

    pub fn sadd(&self, key: String, members: Vec<BulkString>) -> usize {
        let set = self.set.entry(key).or_default();
        let mut count = 0;
        for member in members {
            if set.insert(member) {
                count += 1;
            }
        }
        count
    }

    pub fn sismember(&self, key: &str, member: &BulkString) -> bool {
        self.set.get(key).map_or(false, |set| set.contains(member))
    }
}
