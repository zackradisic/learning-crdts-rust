mod proto;
use anyhow::{anyhow, Context, Result};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use proto::{
    ClientBound, ClientBoundCursor, ClientBoundSync, ClientBoundUpdate, ServerBound,
    ServerBoundCursor, ServerBoundSync, ServerBoundUpdate,
};
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;

use std::sync::{atomic::AtomicU64, Arc};

use convergent_experiment_protocol::{ReplicaId, Square, SquareId};
use sypytkowski_blog::delta_state::awormap::{AWORMap, Deltas};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Mutex, RwLock},
};

struct Ctx {
    state: Arc<RwLock<AWORMap<SquareId, Square>>>,
    connections: Arc<RwLock<Vec<Client>>>,
    id_counter: AtomicU64,
}

impl Ctx {
    fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(AWORMap::default())),
            connections: Arc::new(RwLock::new(Vec::new())),
            id_counter: 0.into(),
        }
    }

    fn new_id(&self) -> u64 {
        self.id_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    async fn add_connection(&self, client: Client) {
        self.connections.write().await.push(client);
    }

    async fn broadcast_cursors(&self, replica: ReplicaId) {
        let mut cursors = vec![];
        for client in self.connections.read().await.iter() {
            let cursor = client.cursor.read().await;
            cursors.push((cursor.0, cursor.1, client.id));
        }

        self.broadcast_msg(
            ClientBound::Cursor(ClientBoundCursor { pos: cursors }),
            self.connections.write().await.iter_mut(),
        )
        .await;
    }

    async fn remove_connection(&self, id: ReplicaId) {
        self.connections.write().await.retain(|c| c.id != id);
    }

    async fn get_state(&self) -> AWORMap<SquareId, Square> {
        self.state.read().await.clone()
    }

    async fn handle_cursor(&self, origin: ReplicaId, (x, y): (f32, f32)) {
        match self
            .connections
            .read()
            .await
            .iter()
            .find(|c| c.id == origin)
        {
            Some(c) => {
                *(c.cursor.write().await) = (x, y);
            }
            None => (),
        };

        self.broadcast_msg(
            ClientBound::Cursor(ClientBoundCursor {
                pos: vec![(x, y, origin)],
            }),
            self.connections
                .write()
                .await
                .iter_mut()
                .filter(|c| c.id != origin),
        )
        .await;
    }

    async fn handle_update(&self, origin: ReplicaId, deltas: Deltas<SquareId, Square>) {
        self.state.write().await.merge_delta(deltas.clone());
        println!("DELTAS: {:#?}", deltas);
        println!("STATE: {:#?}", self.state.read().await.clone());
        self.broadcast_msg(
            ClientBound::Update(ClientBoundUpdate { deltas }),
            self.connections
                .write()
                .await
                .iter_mut()
                .filter(|c| c.id != origin),
        )
        .await;
    }

    async fn handle_sync(
        &self,
        remote_state: AWORMap<SquareId, Square>,
    ) -> AWORMap<SquareId, Square> {
        let mut state = self.state.write().await;
        *state = state.merge(&remote_state);
        self.broadcast_msg(
            ClientBound::Sync(ClientBoundSync {
                state: state.clone(),
            }),
            self.connections.write().await.iter_mut(),
        )
        .await;
        state.clone()
    }

    async fn broadcast_msg<'a, C: Iterator<Item = &'a mut Client>>(
        &self,
        msg: ClientBound,
        clients: C,
    ) {
        let mut buf = Vec::with_capacity(128);
        msg.encode_msgpack(&mut buf);

        for client in clients {
            let result = client
                .write
                .lock()
                .await
                .send(Message::Binary(buf.clone()))
                .await;

            match result {
                Err(e) => {
                    println!("Error sending message to client ({:?}): {}", client.id, e);
                }
                _ => (),
            }
        }
    }
}

#[derive(Clone)]
struct Client {
    id: ReplicaId,
    write: Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>,
    cursor: Arc<RwLock<(f32, f32)>>,
}

impl Client {
    pub async fn new(
        stream: TcpStream,
        ctx: Arc<Ctx>,
    ) -> Result<(Self, SplitStream<WebSocketStream<TcpStream>>)> {
        let ws_stream = tokio_tungstenite::accept_async(stream)
            .await
            .with_context(|| "Error during the websocket handshake occurred")?;

        let (mut w, mut r) = ws_stream.split();

        let msg: ServerBound = r
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("Client did not send a message after connecting"))?
            .with_context(|| "Error reading init message from client")?
            .try_into()
            .with_context(|| "Error parsing init message from client")?;

        let id = match msg {
            ServerBound::Sync(ServerBoundSync {
                replica_id,
                state: remote_state,
            }) => {
                let state = if remote_state.len() == 0 {
                    ctx.get_state().await
                } else {
                    ctx.handle_sync(remote_state).await
                };

                let mut buf = Vec::with_capacity(128);
                ClientBound::Sync(ClientBoundSync { state }).encode_msgpack(&mut buf);
                w.send(Message::Binary(buf)).await.unwrap();

                replica_id
            }
            _ => {
                return Err(anyhow!(
                    "Client did not send a sync message after connecting"
                ))
            }
        };

        Ok((
            Self {
                id,
                write: Arc::new(Mutex::new(w)),
                cursor: Arc::new(RwLock::new((0.0, 0.0))),
            },
            r,
        ))
    }

    pub async fn listen(
        replica: ReplicaId,
        mut r: SplitStream<WebSocketStream<TcpStream>>,
        ctx: Arc<Ctx>,
    ) -> Result<()> {
        while let Some(msg) = r.next().await {
            let msg = msg?;
            let msg = proto::ServerBound::try_from(msg)?;
            match msg {
                proto::ServerBound::Sync(ServerBoundSync { replica_id, state }) => {
                    ctx.handle_sync(state).await;
                }
                proto::ServerBound::Update(ServerBoundUpdate { deltas }) => {
                    ctx.handle_update(replica, deltas).await;
                }
                ServerBound::Cursor(ServerBoundCursor { pos }) => {
                    ctx.handle_cursor(replica, pos).await;
                }
            }
        }

        Ok(())
    }

    pub async fn send(&self, msg: proto::ServerBound) {
        let mut w = self.write.lock().await;
        let mut buf = Vec::new();
        msg.encode_msgpack(&mut buf);
        w.send(tungstenite::Message::Binary(buf)).await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    // Create the event loop and TCP listener we'll accept connections on.
    let addr = "127.0.0.1:6969";
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    let ctx = Arc::new(Ctx::new());

    println!("Listening on: {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        let ctx = ctx.clone();

        let (client, r) = match Client::new(stream, ctx.clone()).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to create client ({}): {:?}", addr, e);
                continue;
            }
        };

        let replica = client.id;
        ctx.add_connection(client.clone()).await;
        ctx.broadcast_cursors(replica).await;

        tokio::spawn(async move {
            match Client::listen(replica, r, ctx.clone()).await {
                Err(e) => {
                    eprintln!("Error handling client ({:?}): {:?}", client.id, e)
                }
                _ => (),
            };
            ctx.remove_connection(client.id).await;
        });
    }
}
