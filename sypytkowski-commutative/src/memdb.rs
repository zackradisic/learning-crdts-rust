use std::{collections::BTreeMap, sync::Arc};

use crate::{Crdt, Event, ReplicationState, Store};
use async_trait::async_trait;
use futures::{future::BoxFuture, FutureExt};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct InMemoryDb<C: Crdt> {
    pub state: Arc<RwLock<Option<ReplicationState<C>>>>,
    pub events: Arc<RwLock<BTreeMap<u64, Event<C::EData>>>>,
}

impl<C: Crdt> Default for InMemoryDb<C> {
    fn default() -> Self {
        Self {
            state: Arc::new(RwLock::new(None)),
            events: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
}

unsafe impl<C: Crdt> Send for InMemoryDb<C> {}

#[async_trait]
impl<C: Crdt> Store<C> for InMemoryDb<C> {
    async fn save_snapshot(&mut self, state: ReplicationState<C>) {
        let mut current_state = self.state.write().await;
        *current_state = Some(state);
    }

    async fn load_snapshot(&mut self) -> Option<ReplicationState<C>> {
        self.state.read().await.clone()
    }

    async fn load_events<'a>(
        &'a mut self,
        start_seq: u64,
    ) -> futures::stream::FuturesOrdered<BoxFuture<'a, Event<C::EData>>> {
        let events_map = self.events.read().await;
        let events = events_map.range(start_seq..).map(|(_, event)| {
            let new_event = event.clone();
            async { new_event }.boxed()
        });

        futures::stream::FuturesOrdered::from_iter(events)
    }

    async fn save_events<I: Iterator<Item = crate::Event<<C as Crdt>::EData>> + Send>(
        &mut self,
        events: I,
    ) {
        let mut events_map = self.events.write().await;
        for event in events {
            events_map.insert(event.local_seq, event);
        }
    }
}
