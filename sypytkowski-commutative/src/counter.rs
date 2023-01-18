use crate::Crdt;

#[derive(Clone, Debug, Default)]
pub struct Counter {
    val: i64,
}

impl Crdt for Counter {
    type State = i64;

    type EData = i64;

    type Cmd = i64;

    fn query(&self) -> Self::State {
        self.val
    }

    fn prepare(&self, op: Self::Cmd) -> Self::EData {
        op
    }

    fn effect(&mut self, event: crate::Event<Self::EData>) {
        self.val = self.val.saturating_add(event.data);
    }
}

#[cfg(test)]
mod test {

    use crate::{
        counter::Counter, memdb::InMemoryDb, protocol::Protocol, replicate, ReplicaId, Replicator,
    };

    #[tokio::test]
    async fn commutativity() {
        let alice_id = ReplicaId(0);
        let bob_id = ReplicaId(1);
        let mut alice = Replicator::new(alice_id, InMemoryDb::<Counter>::default()).await;
        let mut bob = Replicator::new(bob_id, InMemoryDb::<Counter>::default()).await;

        let _ = alice.send(Protocol::Command(34)).await;
        let _ = bob.send(Protocol::Command(35)).await;

        replicate(&mut alice, &mut bob).await;
        replicate(&mut bob, &mut alice).await;

        let alice_value = alice.query();
        let bob_value = bob.query();

        assert_eq!(alice_value, 69);
        assert_eq!(alice_value, bob_value)
    }

    // use proptest::{collection::btree_map, prelude::*};

    // fn replicaid_strategy() -> impl Strategy<Value = ReplicaId> {
    //     any::<u64>().prop_map(ReplicaId)
    // }

    // fn counter_strategy() -> impl Strategy<Value = Replicator<Counter, InMemoryDb<Counter>>> {
    //     (replicaid_strategy(), any::<i64>())
    //         .prop_map(|(id, val)| Replicator::new(id, InMemoryDb::<Counter>::default()).await)
    // }

    // proptest! {
    //     // #![proptest_config(ProptestConfig{ cases: 5, ..Default::default()})]
    //     #![proptest_config(ProptestConfig{ ..Default::default()})]

    //     #[test]
    //     fn commutativity(a in counter_strategy(), b in counter_strategy()) {

    //         let ab = a.merge(&b);
    //         let ba = b.merge(&a);

    //         assert_eq!(ab, ba)
    //     }
    // }
}
