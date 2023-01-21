use std::cmp::Ordering;

use crate::{Crdt, ReplicaId, VTime};

#[derive(Clone, Debug, Default)]
pub struct LWWRegister<V> {
    id: ReplicaId,
    time: VTime,
    value: Option<V>,
}

impl<V> LWWRegister<V> {
    pub fn new(id: ReplicaId) -> Self {
        Self {
            id,
            time: VTime::default(),
            value: None,
        }
    }
}

impl<V: Default + Clone + Send + Sync + std::fmt::Debug> Crdt for LWWRegister<V> {
    type State = Option<V>;

    type EData = Option<V>;

    type Cmd = Option<V>;

    fn query(&self) -> Self::State {
        self.value.clone()
    }

    fn prepare(&self, op: Self::Cmd) -> Self::EData {
        op
    }

    fn effect(&mut self, event: crate::Event<Self::EData>) {
        let value = event.data;
        let at = event.version;

        match self.time.partial_cmp(&at) {
            Some(Ordering::Less) => {
                self.time = at;
                self.value = value;
            }
            None => {
                if self.id >= event.origin {
                    self.time = at;
                    self.value = value;
                }
            }
            // These aren't possible, due to RCB.
            // Ordering::Equal can't be seen because RCB keeps duplicates in check
            // Ordering::Greater can't be seen because RCB makes sure events which are strictly greater
            // won't be processed first.
            // More info at the end of the Multi Value Register section of the article: https://bartoszsypytkowski.com/operation-based-crdts-registers-and-sets/
            Some(Ordering::Equal | Ordering::Greater) => {
                #[cfg(debug_assertions)]
                panic!("Ordering::Equal | Ordering::Greater is impossible due to RCB")
            }
        }
    }
}

#[cfg(test)]
mod test {

    use crate::{
        lwwreg::LWWRegister, memdb::InMemoryDb, protocol::Protocol, replicate, ReplicaId,
        Replicator,
    };

    #[tokio::test]
    async fn test() {
        type LWW<'a> = LWWRegister<&'a str>;

        let alice_id = ReplicaId(0);
        let bob_id = ReplicaId(1);
        let mut alice =
            Replicator::new(alice_id, LWW::new(alice_id), InMemoryDb::<LWW>::default()).await;
        let mut bob = Replicator::new(bob_id, LWW::new(bob_id), InMemoryDb::<LWW>::default()).await;

        let _ = alice.send(Protocol::Command(Some("nice"))).await;
        let _ = bob.send(Protocol::Command(Some("nah"))).await;

        replicate(&mut alice, &mut bob).await;
        replicate(&mut bob, &mut alice).await;

        let alice_value = alice.query();
        let bob_value = bob.query();

        assert_eq!(alice_value, Some("nice"));
        assert_eq!(alice_value, bob_value)
    }
}
