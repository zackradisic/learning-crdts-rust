use std::{collections::HashSet, hash::Hash};

use crate::{Crdt, VTime};

#[derive(Clone, Debug)]
pub struct ORSet<V: Hash> {
    values: HashSet<(V, ClockWrapper)>,
}

#[derive(Debug, Clone)]
pub enum Command<V: Hash> {
    Add(V),
    Remove(V),
}

#[derive(Debug, Clone, Hash)]
pub struct ClockWrapper(VTime);
impl Eq for ClockWrapper {
    fn assert_receiver_is_total_eq(&self) {}
}
impl PartialEq for ClockWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0.map == other.0.map
    }
}

#[derive(Debug, Clone)]
pub enum Op<V: Hash> {
    Added(V),
    Removed(HashSet<ClockWrapper>),
}

impl<V: Hash> ORSet<V> {
    pub fn new() -> Self {
        Self {
            values: HashSet::new(),
        }
    }
}

impl<V: Eq + Hash + Clone + Send + Sync + std::fmt::Debug> Crdt for ORSet<V> {
    type State = HashSet<V>;

    type Cmd = Command<V>;

    type EData = Op<V>;

    fn query(&self) -> Self::State {
        self.values.iter().map(|(v, _)| v.clone()).collect()
    }

    fn prepare(&self, op: Self::Cmd) -> Self::EData {
        match op {
            Command::Add(val) => Op::Added(val),
            Command::Remove(val) => Op::Removed(
                self.values
                    .iter()
                    .filter_map(|(v, clock)| if v == &val { Some(clock.clone()) } else { None })
                    .collect(),
            ),
        }
    }

    fn effect(&mut self, event: crate::Event<Self::EData>) {
        match event.data {
            Op::Added(val) => {
                self.values.insert((val, ClockWrapper(event.version)));
            }
            Op::Removed(removed) => {
                self.values
                    .drain_filter(|(_, clock)| removed.contains(clock));
            }
        }
    }
}

#[cfg(test)]
mod test {

    use std::collections::HashSet;

    use crate::{
        memdb::InMemoryDb,
        orset::{Command, ORSet},
        protocol::Protocol,
        replicate, ReplicaId, Replicator,
    };

    #[tokio::test]
    async fn add() {
        type Crdt<'a> = ORSet<&'a str>;

        let alice_id = ReplicaId(0);
        let bob_id = ReplicaId(1);
        let mut alice = Replicator::new(alice_id, Crdt::new(), InMemoryDb::<Crdt>::default()).await;
        let mut bob = Replicator::new(bob_id, Crdt::new(), InMemoryDb::<Crdt>::default()).await;

        let _ = alice.send(Protocol::Command(Command::Add("nice"))).await;
        let _ = bob.send(Protocol::Command(Command::Add("nah"))).await;

        replicate(&mut alice, &mut bob).await;
        replicate(&mut bob, &mut alice).await;

        let alice_value = alice.query();
        let bob_value = bob.query();

        assert_eq!(alice_value, HashSet::from_iter(["nice", "nah"]));
        assert_eq!(alice_value, bob_value)
    }

    #[tokio::test]
    async fn remove() {
        type Crdt<'a> = ORSet<&'a str>;

        let alice_id = ReplicaId(0);
        let bob_id = ReplicaId(1);
        let mut alice = Replicator::new(alice_id, Crdt::new(), InMemoryDb::<Crdt>::default()).await;
        let mut bob = Replicator::new(bob_id, Crdt::new(), InMemoryDb::<Crdt>::default()).await;

        let _ = alice.send(Protocol::Command(Command::Add("nice"))).await;
        let _ = bob.send(Protocol::Command(Command::Add("nah"))).await;

        replicate(&mut alice, &mut bob).await;
        replicate(&mut bob, &mut alice).await;

        let _ = alice.send(Protocol::Command(Command::Remove("nah"))).await;
        replicate(&mut alice, &mut bob).await;
        replicate(&mut bob, &mut alice).await;

        let alice_value = alice.query();
        let bob_value = bob.query();

        assert_eq!(alice_value, HashSet::from_iter(["nice"]));
        assert_eq!(alice_value, bob_value)
    }
}
