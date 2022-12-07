#![feature(option_get_or_insert_default)]

pub mod delta_state;
pub mod state;
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReplicaId(u64);

impl From<u64> for ReplicaId {
    fn from(val: u64) -> Self {
        Self(val)
    }
}

pub struct ReplicaGenerator {
    count: u64,
}

impl ReplicaGenerator {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    pub fn gen(&mut self) -> ReplicaId {
        let ret = self.count;
        self.count += 1;
        ReplicaId(ret)
    }
}
