use crate::{Event, EventData, ReplicaId, VTime};

#[derive(Debug)]
pub enum Protocol<Cmd: std::fmt::Debug, EData: EventData> {
    // Query,
    // QueryResponse(State),
    Command(Cmd),
    Connect(Connect),
    Replicate(Replicate),
    Replicated(Replicated<EData>),
    Noop,
}

#[derive(Debug)]
pub struct Connect {
    pub replica_id: ReplicaId,
}

#[derive(Debug)]
pub struct Replicate {
    pub seq_nr: u64,
    pub max_count: u64,
    pub filter: VTime,
    pub reply_to: ReplicaId,
}

#[derive(Debug)]
pub struct Replicated<D: EventData> {
    pub from: ReplicaId,
    pub to_seq_nr: u64,
    pub events: Vec<Event<D>>,
}
