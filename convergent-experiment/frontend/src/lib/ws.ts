import type { StateMachineDef, SelectStates } from "tyfsm";
import { Exports as Runtime } from "./proto";
import create from "tyfsm";
import { decode } from "@msgpack/msgpack";
import {
  ClientBound,
  decodeClientBound,
  encodeServerBound,
  ServerBound,
} from "./rpc";
import { ReplicaId } from "./proto/types";
import { useAppState } from "./state";

export type WebsocketMachine = StateMachineDef<
  {
    idle: { addr: string; runtime?: Runtime; replicaId?: ReplicaId };
    ready: { addr: string; runtime: Runtime; replicaId: ReplicaId };
    connecting: {
      addr: string;
      runtime: Runtime;
      socket: WebSocket;
      replicaId: ReplicaId;
    };
    connected: {
      addr: string;
      runtime: Runtime;
      socket: WebSocket;
      replicaId: ReplicaId;
    };
    error: {
      addr: string;
      runtime: Runtime;
      errorMessage: string;
    };
  },
  {
    idle: ["ready"];
    ready: ["connecting"];
    connecting: ["error", "connected"];
    connected: ["ready", "error"];
    error: ["idle"];
  }
>;

export type State<K extends WebsocketMachine["allStates"]> = SelectStates<
  WebsocketMachine,
  K
>;

type Actions = {
  setRuntime: (
    state: State<"idle">,
    runtime: Runtime,
    replicaId: ReplicaId
  ) => State<"ready">;
  connect: (state: State<"ready">) => State<"connecting">;
  send: (state: State<"connected">, payload: ServerBound) => State<"connected">;
  disconnect: (state: State<"connecting" | "connected">) => State<"ready">;
};

// Create the initial state
const initial: State<"idle"> = {
  kind: "idle",
  addr: "ws://localhost:6969",
};

export const useWebsocketStore = create<WebsocketMachine, Actions>(
  initial,
  (get, transition) => ({
    setRuntime(state, runtime, replicaId) {
      return transition(state.kind, "ready", {
        addr: state.addr,
        runtime,
        replicaId,
      });
    },
    connect(idleState) {
      const socket = new WebSocket(idleState.addr);

      socket.addEventListener("error", (e) => {
        console.error("WS error", e);
        const currentState = get();
        if (currentState.kind === "connecting") {
          transition(currentState.kind, "error", {
            addr: currentState.addr,
            errorMessage: "Failed to connect",
            runtime: currentState.runtime,
          });
        }
      });

      socket.addEventListener("open", () => {
        const currentState = get();
        if (currentState.kind === "connecting") {
          const msg = encodeServerBound({
            type: "sync",
            state: currentState.runtime.get!(),
            replicaId: idleState.replicaId,
          });
          currentState.socket.send(msg);
          transition(currentState.kind, "connected", {
            socket,
            addr: currentState.addr,
            runtime: currentState.runtime,
            replicaId: currentState.replicaId,
          });
        }
      });

      socket.addEventListener("message", async (e) => {
        const currentState = get();
        if (currentState.kind === "connected") {
          const arrayBuf = await e.data.arrayBuffer();
          const clientBound = decodeClientBound(arrayBuf);

          if (clientBound.type !== "cursor") {
            console.log("Client bound message", clientBound);
            useAppState.getState().setPrevClientMsg(clientBound);
          }

          switch (clientBound.type) {
            case "sync": {
              useAppState.getState().remote.merge(clientBound.state);
              break;
            }
            case "update": {
              useAppState.getState().remote.mergeDeltas(clientBound.deltas);
              break;
            }
            case "cursor": {
              useAppState.getState().remote.setCursors(clientBound.pos);
              break;
            }
          }
        }
      });

      return transition(idleState.kind, "connecting", {
        socket,
        addr: idleState.addr,
        runtime: idleState.runtime,
        replicaId: idleState.replicaId,
      });
    },
    send(state, payload) {
      const msg = encodeServerBound(payload);
      state.socket.send(msg);
      return state;
    },
    disconnect(state) {
      state.socket.close();
      return transition(state.kind, "ready", {
        addr: state.addr,
        runtime: state.runtime,
        replicaId: state.replicaId,
      });
    },
  })
);
