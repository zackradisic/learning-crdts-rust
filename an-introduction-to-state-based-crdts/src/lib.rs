pub mod grow_counter;
pub mod or_set;
pub mod pn_counter;
pub mod vector_clock;

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
