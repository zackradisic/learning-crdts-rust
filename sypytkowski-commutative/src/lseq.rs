use std::{cmp::Ordering, fmt::Debug, io::Write};

use crate::{Crdt, ReplicaId};

#[derive(Clone)]
pub struct LSeq<V> {
    values: Vec<Vertex<V>>,
    id: ReplicaId,
}

#[derive(PartialEq, Clone, Debug)]
pub struct VPtr {
    sequence: Vec<u8>,
    id: ReplicaId,
}

#[derive(Clone)]
pub struct Vertex<V>(VPtr, V);

#[derive(Clone, Debug)]
pub enum Command<V: Debug> {
    Insert(u32, V),
    RemoveAt(u32),
}

#[derive(Clone, Debug)]
pub enum Operation<V: Debug> {
    Inserted(VPtr, V),
    Removed(VPtr),
}

impl<V> LSeq<V> {
    pub fn new(id: ReplicaId) -> Self {
        Self { values: vec![], id }
    }
}

impl<V: Sync + Send + Clone + Debug> Crdt for LSeq<V> {
    type State = Vec<V>;

    type Cmd = Command<V>;

    type EData = Operation<V>;

    fn query(&self) -> Self::State {
        self.values.iter().map(|v| v.1.clone()).collect()
    }

    fn prepare(&self, op: Self::Cmd) -> Self::EData {
        match op {
            Command::Insert(i, value) => {
                let lo = if i == 0 {
                    &[]
                } else {
                    self.values[i as usize - 1].0.sequence.as_slice()
                };

                let hi = if i == self.values.len() as u32 {
                    &[]
                } else {
                    self.values[i as usize].0.sequence.as_slice()
                };

                let mut sequence = vec![];
                VPtr::generate_seq(&mut sequence, lo, hi);

                Operation::Inserted(
                    VPtr {
                        sequence,
                        id: self.id,
                    },
                    value,
                )
            }
            Command::RemoveAt(i) => {
                let ptr = self.values[i as usize].0.clone();

                Operation::Removed(ptr)
            }
        }
    }

    fn effect(&mut self, event: crate::Event<Self::EData>) {
        match event.data {
            Operation::Inserted(ptr, value) => {
                let idx = self
                    .values
                    .binary_search_by(|Vertex(vptr, _)| VPtr::compare(vptr, &ptr))
                    .unwrap_or_else(|val| val);

                self.values.insert(idx, Vertex(ptr, value));
            }
            Operation::Removed(ptr) => {
                let idx = match self
                    .values
                    .binary_search_by(|Vertex(vptr, _)| VPtr::compare(vptr, &ptr))
                {
                    Ok(idx) => idx,
                    _ => {
                        // This could happen if two clients concurrently remove the same element (see remove test case below)
                        return;
                    }
                };

                self.values.remove(idx);
            }
        }
    }
}

impl VPtr {
    fn to_string_impl<W: std::io::Write>(
        &self,
        mut bw: std::io::BufWriter<W>,
    ) -> std::io::Result<()> {
        for byte in self.sequence.iter().take((self.sequence.len() - 1).max(0)) {
            write!(bw, "{}.", *byte)?;
        }

        if !self.sequence.is_empty() {
            write!(bw, "{}", self.sequence.last().unwrap())?;
        }

        write!(bw, ":{}", self.id.0)?;

        Ok(())
    }

    pub fn compare(a: &Self, b: &Self) -> Ordering {
        match a.sequence.len().cmp(&b.sequence.len()) {
            Ordering::Equal => return a.id.cmp(&b.id),
            _ => (),
        }
        a.sequence.cmp(&b.sequence)
    }

    pub fn generate_seq(acc: &mut Vec<u8>, lo: &[u8], hi: &[u8]) {
        let mut i = 0;
        loop {
            let min = lo.get(i).copied().unwrap_or(0);
            let max = hi.get(i).copied().unwrap_or(u8::MAX);

            if min + 1 < max {
                acc.push(min + 1);
                return;
            }

            acc.push(min);
            i += 1;
        }
    }
}

#[cfg(test)]
mod test {

    use std::collections::HashSet;

    use crate::{
        lseq::{Command, LSeq},
        memdb::InMemoryDb,
        protocol::Protocol,
        replicate, ReplicaId, Replicator,
    };

    #[tokio::test]
    async fn add() {
        type Crdt<'a> = LSeq<&'a str>;

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

        assert_eq!(alice_value, vec!["nice", "nah"]);
        assert_eq!(alice_value, bob_value)
    }

    #[tokio::test]
    async fn remove() {
        type Crdt<'a> = LSeq<&'a str>;

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

        assert_eq!(alice_value, vec!["nah"]);
        assert_eq!(alice_value, bob_value)
    }
}
