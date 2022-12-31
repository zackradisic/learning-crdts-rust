import { Square, SquareId } from "./proto/types";
import { useAppState } from "./state";
import { useWebsocketStore } from "./ws";

export const setSquare = (id: SquareId, square: Square) => {
  useAppState.getState().local.setSquare(id, square);
  const state = useWebsocketStore.getState();
  if (state.kind !== "connected") {
    console.error("Websocket not connected");
    return;
  }
  const deltas = useAppState.getState().deltas();
  useWebsocketStore.actions.send(state, { type: "update", deltas });
};

export const updateCursor = (x: number, y: number) => {
  const state = useWebsocketStore.getState();
  if (state.kind !== "connected") {
    // console.error("Websocket not connected");
    return;
  }
  useWebsocketStore.actions.send(state, { type: "cursor", pos: [x, y] });
};

export const removeSquare = (id: SquareId) => {
  useAppState.getState().local.deleteSquare(id);
  const state = useWebsocketStore.getState();
  if (state.kind !== "connected") {
    console.error("Websocket not connected");
    return;
  }
  const deltas = useAppState.getState().deltas();
  useWebsocketStore.actions.send(state, { type: "update", deltas });
};
