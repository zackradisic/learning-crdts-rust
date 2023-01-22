use crate::{Crdt, ReplicaId};

use std::fmt::Debug;

#[derive(Clone)]
pub struct Rga<V> {
    values: Vec<Vertex<V>>,
    sequencer: VPtr,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Copy)]
pub struct VPtr(u64, ReplicaId);

#[derive(Clone, Debug)]
pub struct Vertex<V>(VPtr, Option<V>);

#[derive(Debug, Clone)]
pub enum Command<V> {
    Insert(u32, V),
    RemoveAt(u32),
}

#[derive(Clone, Debug)]
pub enum Operation<V> {
    Inserted {
        predecessor: VPtr,
        ptr: VPtr,
        val: V,
    },
    Removed {
        pos: VPtr,
    },
}

impl<V: Sync + Send + Clone + Debug> Rga<V> {
    pub fn new(replica_id: ReplicaId) -> Self {
        Self {
            values: vec![Vertex(VPtr(0, ReplicaId(u64::MAX)), None)],
            sequencer: VPtr(0, replica_id),
        }
    }

    fn shift(&self, offset: usize, ptr: VPtr) -> usize {
        if offset >= self.values.len() {
            return offset;
        }

        let Vertex(successor, _) = self.values[offset];
        if successor < ptr {
            return offset;
        }

        self.shift(offset + 1, ptr)
    }

    fn index_of_vptr(&self, ptr: VPtr) -> usize {
        match self
            .values
            .iter()
            .enumerate()
            .find(|(_, Vertex(vptr, _))| vptr == &ptr)
        {
            Some((i, _)) => i,
            None => {
                panic!(
                    "COULDN'T FIND VPTR {:?} {:?} {:?}",
                    self.sequencer.1, ptr, self.values
                );
            }
        }
        // .map(|v| v.0)
    }

    fn apply_inserted(&mut self, predecessor: VPtr, ptr: VPtr, value: V) {
        // In the case that we didn't find the vptr then just
        let predecessor_idx = self.index_of_vptr(predecessor); //.unwrap_or(self.values.len());

        println!("PREDECESSOR {}", predecessor_idx);
        let insert_idx = self.shift(predecessor_idx + 1, ptr);
        println!("SHIFT!! {} {:?}", insert_idx, self.sequencer.1);

        let VPtr(seq, id) = self.sequencer.incr();
        let next_seq = VPtr(seq.max(ptr.0), id);

        println!(
            "INSERTING {} {:?} {:?} {:?}",
            insert_idx, self.sequencer.1, value, self.values
        );
        self.values.insert(insert_idx, Vertex(ptr, Some(value)));
        self.sequencer = next_seq;
        println!("FINAL INSERT {:?}: {:?}", self.sequencer.1, self.values);
    }

    fn apply_removed(&mut self, pos: VPtr) {
        // let index = match self.index_of_vptr(pos) {
        //     Some(idx) => idx,
        //     // either was already deleted or never existed
        //     None => return,
        // };
        let index = self.index_of_vptr(pos);
        self.values[index].1 = None;
        println!("FINAL REMOVE {:?}: {:?}", self.sequencer.1, self.values);
    }

    fn index_including_tombstones(&self, mut i: u32) -> usize {
        let mut offset = 1;
        for vertex in self.values.iter().skip(1) {
            if i == 0 {
                return offset;
            }

            if !vertex.is_tombstone() {
                i -= 1;
            }

            offset += 1
        }
        return offset + i as usize;
    }
}

impl<V: Sync + Send + Clone + Debug> Crdt for Rga<V> {
    type State = Vec<V>;

    type Cmd = Command<V>;

    type EData = Operation<V>;

    fn query(&self) -> Self::State {
        self.values.iter().filter_map(|a| a.1.clone()).collect()
    }

    fn prepare(&self, op: Self::Cmd) -> Self::EData {
        match op {
            Command::Insert(i, val) => {
                let index = self.index_including_tombstones(i);
                let predecessor = self.values[index - 1].0;
                let at = self.sequencer.incr();

                Operation::Inserted {
                    predecessor,
                    ptr: at,
                    val,
                }
            }
            Command::RemoveAt(i) => {
                let index = self.index_including_tombstones(i);
                let pos = self.values[index].0;
                Operation::Removed { pos }
            }
        }
    }

    fn effect(&mut self, event: crate::Event<Self::EData>) {
        match event.data {
            Operation::Inserted {
                predecessor,
                ptr,
                val,
            } => {
                self.apply_inserted(predecessor, ptr, val);
            }
            Operation::Removed { pos } => self.apply_removed(pos),
        }
    }
}

impl<V> Vertex<V> {
    fn is_tombstone(&self) -> bool {
        self.1.is_none()
    }
}

impl VPtr {
    fn incr(self) -> Self {
        VPtr(self.0 + 1, self.1)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        memdb::InMemoryDb,
        protocol::Protocol,
        replicate,
        rga::{Command, Rga},
        ReplicaId, Replicator,
    };

    use super::VPtr;

    #[test]
    fn vptr_structural_comparison() {
        let a = VPtr(0, ReplicaId(0));
        let b = VPtr(0, ReplicaId(1));

        assert!(a < b)
    }

    #[tokio::test]
    async fn add() {
        type Crdt<'a> = Rga<&'a str>;

        let alice_id = ReplicaId(0);
        let bob_id = ReplicaId(1);
        let mut alice =
            Replicator::new(alice_id, Crdt::new(alice_id), InMemoryDb::<Crdt>::default()).await;
        let mut bob =
            Replicator::new(bob_id, Crdt::new(bob_id), InMemoryDb::<Crdt>::default()).await;

        let _ = alice
            .send(Protocol::Command(Command::Insert(0, "nice")))
            .await;
        let _ = bob.send(Protocol::Command(Command::Insert(0, "nah"))).await;

        replicate(&mut alice, &mut bob).await;
        replicate(&mut bob, &mut alice).await;

        let alice_value = alice.query();
        let bob_value = bob.query();

        assert_eq!(alice_value, vec!["nah", "nice"]);
        assert_eq!(alice_value, bob_value)
    }

    #[tokio::test]
    async fn remove() {
        type Crdt<'a> = Rga<&'a str>;

        let alice_id = ReplicaId(0);
        let bob_id = ReplicaId(1);
        let mut alice =
            Replicator::new(alice_id, Crdt::new(alice_id), InMemoryDb::<Crdt>::default()).await;
        let mut bob =
            Replicator::new(bob_id, Crdt::new(bob_id), InMemoryDb::<Crdt>::default()).await;

        let _ = alice
            .send(Protocol::Command(Command::Insert(0, "nice")))
            .await;
        let _ = bob.send(Protocol::Command(Command::Insert(0, "nah"))).await;

        replicate(&mut alice, &mut bob).await;
        replicate(&mut bob, &mut alice).await;

        let _ = alice.send(Protocol::Command(Command::RemoveAt(0))).await;
        let _ = bob.send(Protocol::Command(Command::RemoveAt(0))).await;
        replicate(&mut alice, &mut bob).await;
        replicate(&mut bob, &mut alice).await;

        let alice_value = alice.query();
        let bob_value = bob.query();

        // State should be ["nah", "nice"]
        // Alice and Bob both delete "nah"
        assert_eq!(alice_value, vec!["nice"]);
        assert_eq!(alice_value, bob_value)
    }
}
