use convergent_experiment_protocol::{ReplicaId, Square, SquareId};
use serde::{Deserialize, Serialize};
use sypytkowski_blog::delta_state::awormap::{AWORMap, Deltas};
use tungstenite::Message;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum ServerBound {
    Sync(ServerBoundSync),
    Update(ServerBoundUpdate),
    Cursor(ServerBoundCursor),
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServerBoundSync {
    pub replica_id: ReplicaId,
    pub state: AWORMap<SquareId, Square>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServerBoundUpdate {
    pub deltas: Deltas<SquareId, Square>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServerBoundCursor {
    pub pos: (f32, f32),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum ClientBound {
    Sync(ClientBoundSync),
    Update(ClientBoundUpdate),
    Cursor(ClientBoundCursor),
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClientBoundSync {
    pub state: AWORMap<SquareId, Square>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClientBoundUpdate {
    pub deltas: Deltas<SquareId, Square>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClientBoundCursor {
    pub pos: Vec<(f32, f32, ReplicaId)>,
}

impl ServerBound {
    #[inline]
    pub fn encode_msgpack(&self, buf: &mut Vec<u8>) {
        rmp_serde::encode::write_named(buf, self).unwrap();
    }
}
impl ClientBound {
    #[inline]
    pub fn encode_msgpack(&self, buf: &mut Vec<u8>) {
        rmp_serde::encode::write_named(buf, self).unwrap();
    }
}

impl TryFrom<Message> for ServerBound {
    type Error = anyhow::Error;

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        if !value.is_binary() {
            return Err(anyhow::anyhow!(
                "Expected binary message but got: {:?}",
                value
            ));
        }

        let bytes = value.into_data();

        ServerBound::deserialize(&mut rmp_serde::Deserializer::new(&bytes[..]))
            .map_err(|e| anyhow::anyhow!("Failed to deserialize message: {}", e))
    }
}

impl TryFrom<Message> for ClientBound {
    type Error = anyhow::Error;

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        if !value.is_binary() {
            return Err(anyhow::anyhow!("Expected binary message"));
        }

        let bytes = value.into_data();

        ClientBound::deserialize(&mut rmp_serde::Deserializer::new(&bytes[..]))
            .map_err(|e| anyhow::anyhow!("Failed to deserialize message: {}", e))
    }
}

#[cfg(test)]
mod test {
    use super::{ClientBound, ServerBound};

    #[test]
    fn noob() {
        let state = ClientBound::Sync(super::ClientBoundSync {
            ..Default::default()
        });
        let mut buf = Vec::with_capacity(128);
        rmp_serde::encode::write_named(&mut buf, &state).unwrap();
        std::fs::write("./state.bin", buf).unwrap()
    }
}
