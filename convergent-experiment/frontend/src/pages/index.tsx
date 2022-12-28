import { type NextPage } from "next";
import Head from "next/head";
import Link from "next/link";
import { useEffect, useState } from "react";
import Canvas from "../lib/Canvas";
import Cursors from "../lib/Cursor";
import { Square } from "../lib/proto/types";
import { useAppState } from "../lib/state";
import { createRuntime } from "../lib/wasm";
import { useWebsocketStore } from "../lib/ws";

let ran = false;
const Home: NextPage = () => {
  const [replicaId, setReplicaId] = useState<number | undefined>(undefined);
  const { state, actions } = useWebsocketStore();

  useEffect(() => {
    if (replicaId === undefined) return;
    const run = async () => {
      if (typeof window === "undefined") return;
      if (ran) return;
      ran = true;

      const runtime = await createRuntime();
      const { get, set, merge, deltas: getDeltas } = runtime;
      if (!get || !set || !merge || !getDeltas) throw new Error("WTF BRO");

      useAppState.getState().setReady(runtime, replicaId);
      const state = useWebsocketStore.getState();
      if (state.kind === "idle") {
        useWebsocketStore.actions.setRuntime(state, runtime, replicaId);
      }

      window.square = (square: Partial<Square>) => ({
        x: 420,
        y: 420,
        width: 100,
        height: 100,
        ...square,
      });
      window.runtime = runtime;
      window.entries = () => Object.values(get().keys.kernel.entries);
    };

    run();
  }, [replicaId]);

  return (
    <div className="h-screen w-full bg-[#2c2c2c] text-[#b4b4b4]">
      <div className="p-4">
        <div className="mb-2 flex flex-row items-baseline text-[#b4b4b4]">
          <p>Agent ID:</p>
          <input
            className="ml-2 bg-inherit text-white focus:outline-none"
            value={replicaId}
            onChange={(e) => {
              try {
                const val = parseInt(e.target.value);
                setReplicaId(val);
              } catch (_) {}
            }}
          />

          <button
            className={
              "ml-auto rounded bg-[#0c8ce9] px-4 py-2 text-sm text-white"
            }
            onClick={(_) => {
              if (state.kind === "ready") {
                actions.connect(state);
              } else if (state.kind === "connected") {
                actions.disconnect(state);
              }
            }}
          >
            {state.kind === "ready"
              ? "Connect"
              : state.kind === "connected"
              ? "Disconnect"
              : state.kind === "connecting"
              ? "Connecting"
              : "Enter ID"}
          </button>
        </div>
      </div>

      <Canvas />
    </div>
  );
};

export default Home;
