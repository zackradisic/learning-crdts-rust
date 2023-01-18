#![feature(option_get_or_insert_default)]
#![feature(btree_drain_filter)]

pub mod memdb;
pub mod protocol;

pub mod counter;
pub mod lwwreg;

use futures::{future::BoxFuture, stream::FuturesOrdered, StreamExt};
use protocol::{self as proto, Protocol};
use std::{
    cmp::Ordering,
    collections::{btree_map::Entry, BTreeMap},
    ops::Deref,
};

use async_trait::async_trait;

#[async_trait]
pub trait Store<C: Crdt> {
    async fn save_snapshot(&mut self, state: ReplicationState<C>);
    async fn load_snapshot(&mut self) -> Option<ReplicationState<C>>;
    // async fn load_events(&mut self, start_seq: u64) -> Vec<Event<C::EData>>;
    async fn load_events<'a>(
        &'a mut self,
        start_seq: u64,
    ) -> FuturesOrdered<BoxFuture<'a, Event<C::EData>>>;
    async fn save_events<I: Iterator<Item = Event<C::EData>> + Send>(&mut self, events: I);
}

pub trait EventData: Clone + Default + Send + Sync + std::fmt::Debug {}
impl<T: Clone + Default + Send + Sync + std::fmt::Debug> EventData for T {}

pub trait Crdt: Clone + Send + Sync {
    type State;
    type EData: EventData;
    type Cmd: std::fmt::Debug;

    fn query(&self) -> Self::State;
    fn prepare(&self, op: Self::Cmd) -> Self::EData;
    fn effect(&mut self, event: Event<Self::EData>);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ReplicaId(u64);

#[derive(Debug, Clone, Default)]
pub struct VTime {
    map: BTreeMap<ReplicaId, u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Event<D: EventData> {
    origin: ReplicaId,
    origin_seq: u64,
    local_seq: u64,
    version: VTime,
    data: D,
}

#[derive(Default, Debug, Clone)]
pub struct ReplicationState<C>
where
    C: Crdt,
{
    id: ReplicaId,
    seq: u64,
    version: VTime,
    observed: BTreeMap<ReplicaId, u64>,
    crdt: C,
}

unsafe impl<C: Crdt> Send for ReplicationState<C> {}

pub struct ReplicationStatus {
    replica_id: ReplicaId,
}

#[derive(Clone, Debug)]
pub struct Replicator<C, Db>
where
    C: Crdt,
    Db: Store<C>,
{
    store: Db,
    state: ReplicationState<C>,
}

impl<C, Db> Replicator<C, Db>
where
    C: Crdt,
    Db: Store<C>,
{
    pub async fn new(id: ReplicaId, crdt: C, mut store: Db) -> Self {
        let snapshot = store.load_snapshot().await;
        let mut state = snapshot.unwrap_or(ReplicationState {
            id,
            crdt,
            seq: 0,
            version: Default::default(),
            observed: Default::default(),
        });

        while let Some(event) = store.load_events(state.seq + 1).await.next().await {
            state.seq = state.seq.max(event.local_seq);
            state.version.merge(&event.version);
            state.observed.insert(event.origin, event.origin_seq);
            state.crdt.effect(event);
        }

        Self { store, state }
    }

    pub fn query(&mut self) -> C::State {
        self.state.crdt.query()
    }

    pub async fn send(
        &mut self,
        msg: Protocol<C::Cmd, C::EData>,
        // replicating_nodes: &mut BTreeMap<ReplicaId, ReplicationStatus>,
    ) -> Protocol<C::Cmd, C::EData> {
        match msg {
            Protocol::Noop => Protocol::Noop,
            Protocol::Command(cmd) => {
                self.state.seq += 1;
                let seq = self.state.seq;
                self.state.version.increment(self.state.id);

                let data = self.state.crdt.prepare(cmd);
                let event = Event {
                    origin: self.state.id,
                    origin_seq: seq,
                    local_seq: seq,
                    version: self.state.version.clone(),
                    data,
                };

                self.store.save_events(std::iter::once(event.clone())).await;
                self.state.crdt.effect(event);
                Protocol::Noop
            }
            Protocol::Connect(connect) => {
                let seq_nr = self
                    .state
                    .observed
                    .get(&connect.replica_id)
                    .copied()
                    .unwrap_or_default();

                let replicate = proto::Replicate {
                    seq_nr: seq_nr + 1,
                    max_count: 100,
                    filter: self.state.version.clone(),
                    reply_to: self.state.id,
                };

                Protocol::Replicate(replicate)
            }
            Protocol::Replicate(replicate) => {
                let replicated = self
                    .replay(
                        self.state.id,
                        replicate.filter,
                        replicate.seq_nr,
                        replicate.max_count,
                    )
                    .await;
                Protocol::Replicated(replicated)
            }
            Protocol::Replicated(proto::Replicated {
                from,
                to_seq_nr,
                events,
            }) if events.is_empty() => {
                // done replicating
                let observed_seq_nr = self.state.observed.get(&from).copied().unwrap_or_default();
                if to_seq_nr > observed_seq_nr {
                    self.state.observed.insert(from, to_seq_nr);
                    self.store.save_snapshot(self.state.clone()).await;
                }
                Protocol::Noop
            }
            Protocol::Replicated(proto::Replicated {
                from,
                to_seq_nr,
                events,
            }) => {
                let mut new_state = self.state.clone();
                let mut remote_seq_nr = new_state.observed.get(&from).copied().unwrap_or_default();

                let mut to_save = vec![];

                // for all events not seen by the current node, rewrite them to use local sequence nr, update the state
                // and save them in the database
                for e in events.into_iter().filter(|e| self.state.is_unseen(from, e)) {
                    new_state.seq += 1;
                    new_state.version.merge(&e.version);
                    remote_seq_nr = remote_seq_nr.max(e.local_seq);

                    let mut new_event = e.clone();
                    new_event.local_seq = new_state.seq;

                    new_state.crdt.effect(e);
                    new_state.observed.insert(from, remote_seq_nr);
                    to_save.push(new_event);
                }
                self.state = new_state;

                self.store.save_events(to_save.into_iter()).await;
                // let target = replicating_nodes.get(&from);

                // Keep replicating because we set `max_count` to 100 by default so there might
                // be more events to replicate
                Protocol::Replicate(proto::Replicate {
                    seq_nr: to_seq_nr + 1,
                    max_count: 100,
                    filter: self.state.version.clone(),
                    reply_to: self.state.id,
                })
            }
            // Protocol::Query => {
            //     let state = self.state.crdt.query();
            //     Protocol::QueryResponse(state)
            // }
            // Protocol::QueryResponse(_) => Protocol::Noop,
        }
    }

    pub async fn replay(
        &mut self,
        replica_id: ReplicaId,
        filter: VTime,
        seq_nr: u64,
        count: u64,
    ) -> proto::Replicated<<C as Crdt>::EData> {
        let mut i = 0;
        let mut events = vec![];
        let mut last_seq_nr = 0;

        // let foo = self.store.load_events(seq_nr).await.take(20);

        println!(
            "EVENTS LOL! {:?}",
            self.store
                .load_events(seq_nr)
                .await
                .collect::<Vec<_>>()
                .await
        );
        let mut event_stream = self.store.load_events(seq_nr).await.take(count as usize);

        while let Some(e) = event_stream.next().await {
            // println!("NICE: {:?}", e);
            last_seq_nr = last_seq_nr.max(e.local_seq);
            if matches!(
                e.version.partial_cmp(&filter),
                Some(Ordering::Greater) | None
            ) {
                events.push(e);
                i += 1;
            }
            if i >= count {
                break;
            }
        }

        proto::Replicated {
            from: replica_id,
            to_seq_nr: last_seq_nr,
            events,
        }
    }
}

impl VTime {
    pub fn merge(&mut self, other: &Self) {
        for (key, val) in other.iter() {
            match self.map.entry(*key) {
                Entry::Occupied(mut entry) => {
                    *entry.get_mut() = (*entry.get()).max(*val);
                }
                Entry::Vacant(entry) => {
                    entry.insert(*val);
                }
            }
        }
    }

    pub fn increment(&mut self, replica: ReplicaId) {
        *self.map.entry(replica).or_default() += 1;
    }

    fn partial_ord_impl(a: &Self, b: &Self) -> Option<Ordering> {
        let all_keys = a.keys().chain(b.keys());
        all_keys.fold(Some(Ordering::Equal), |prev, key| {
            let va = a.get(key).copied().unwrap_or_default();
            let vb = b.get(key).copied().unwrap_or_default();

            // If all values of corresponding replicas are equal, clocks are equal
            // If all values of a <= all values of b, a is less than b
            // If all values of b >= a, b is greater than a
            // Any other mix is concurrent (returns None)
            match prev {
                Some(Ordering::Equal) if va > vb => Some(Ordering::Greater),
                Some(Ordering::Equal) if va < vb => Some(Ordering::Less),
                Some(Ordering::Less) if va > vb => None,
                Some(Ordering::Greater) if va < vb => None,
                _ => prev,
            }
        })
    }
}

impl PartialOrd for VTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Self::partial_ord_impl(&self, &other)
    }
}

impl PartialEq for VTime {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            Self::partial_ord_impl(self, other),
            Some(std::cmp::Ordering::Equal)
        )
    }
}

impl Deref for VTime {
    type Target = BTreeMap<ReplicaId, u64>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<C> ReplicationState<C>
where
    C: Crdt,
{
    pub fn is_unseen(&self, node_id: ReplicaId, e: &Event<C::EData>) -> bool {
        match self.observed.get(&node_id) {
            Some(&ver) if e.origin_seq <= ver => false,
            _ => {
                matches!(
                    e.version.partial_cmp(&self.version),
                    Some(Ordering::Greater) | None
                )
            }
        }
    }
}

pub async fn replicate<C: Crdt, Db: Store<C>>(
    replica: &mut Replicator<C, Db>,
    from: &mut Replicator<C, Db>,
) {
    let seq_nr = replica
        .state
        .observed
        .get(&from.state.id)
        .copied()
        .unwrap_or(0)
        + 1;

    let initial_replicate_message = Protocol::Replicate(proto::Replicate {
        seq_nr,
        max_count: 100,
        filter: replica.state.version.clone(),
        reply_to: replica.state.id,
    });

    replicate_impl(replica, from, initial_replicate_message).await;
}

async fn replicate_impl<C: Crdt, Db: Store<C>>(
    replica: &mut Replicator<C, Db>,
    from: &mut Replicator<C, Db>,
    initial_replicate_msg: Protocol<C::Cmd, C::EData>,
) {
    let mut replicate_response = initial_replicate_msg;

    loop {
        let replicated_response = from.send(replicate_response).await;
        replicate_response = replica.send(replicated_response).await;
        if let Protocol::Noop = replicate_response {
            break;
        }
    }
}

pub async fn connect<C: Crdt, Db: Store<C>>(
    replica: &mut Replicator<C, Db>,
    to: &mut Replicator<C, Db>,
) {
    let initial_replicate_msg = replica
        .send(Protocol::Connect(proto::Connect {
            replica_id: to.state.id,
        }))
        .await;
    replicate_impl(replica, to, initial_replicate_msg).await;
}
