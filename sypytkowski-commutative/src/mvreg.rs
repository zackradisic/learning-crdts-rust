use std::{cmp::Ordering, collections::BTreeSet};

use crate::{Crdt, VTime};

#[derive(Clone, Default, Debug)]
pub struct MVRegister<V> {
    values: Vec<(VTime, Option<V>)>,
}

impl<V> MVRegister<V> {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }
}

impl<V: Ord + Default + Clone + Send + Sync + std::fmt::Debug> Crdt for MVRegister<V> {
    type State = BTreeSet<V>;

    type Cmd = Option<V>;

    type EData = Option<V>;

    fn query(&self) -> Self::State {
        self.values.iter().filter_map(|(_, v)| v.clone()).collect()
    }

    fn prepare(&self, op: Self::Cmd) -> Self::EData {
        op
    }

    fn effect(&mut self, event: crate::Event<Self::EData>) {
        self.values = std::iter::once((event.version.clone(), event.data))
            .chain(
                self.values
                    .iter()
                    .filter(|(vtime, _)| matches!(vtime.partial_cmp(&event.version), None))
                    .cloned(),
            )
            .collect();
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeSet;

    use crate::{
        memdb::InMemoryDb, mvreg::MVRegister, protocol::Protocol, replicate, ReplicaId, Replicator,
    };

    #[tokio::test]
    async fn test() {
        type Crdt<'a> = MVRegister<&'a str>;

        let alice_id = ReplicaId(0);
        let bob_id = ReplicaId(1);
        let mut alice = Replicator::new(alice_id, Crdt::new(), InMemoryDb::<Crdt>::default()).await;
        let mut bob = Replicator::new(bob_id, Crdt::new(), InMemoryDb::<Crdt>::default()).await;

        let _ = alice.send(Protocol::Command(Some("nice"))).await;
        let _ = bob.send(Protocol::Command(Some("nah"))).await;

        replicate(&mut alice, &mut bob).await;
        replicate(&mut bob, &mut alice).await;

        let alice_value = alice.query();
        let bob_value = bob.query();

        assert_eq!(alice_value, BTreeSet::from_iter(["nice", "nah"]));
        assert_eq!(alice_value, bob_value)
    }
}
