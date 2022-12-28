import { Exports as WasmRuntime } from "./proto";
import type {
  AWORMap,
  Deltas,
  ReplicaId,
  Square,
  SquareId,
} from "./proto/types";
import create from "zustand";

type Base = {
  squares: Record<SquareId, Square>;
  cursors: Record<ReplicaId, [x: number, y: number]>;
};

type Actions = {
  setReady: (wasm: WasmRuntime, replicaId: ReplicaId) => void;
  deltas: () => Deltas<SquareId, Square>;
  local: {
    setSquare: (id: SquareId, square: Square) => void;
  };
  remote: {
    merge: (state: AWORMap<SquareId, Square>) => void;
    mergeDeltas: (crdt: Deltas<SquareId, Square>) => void;
    setCursors: (cursors: [x: number, y: number, id: ReplicaId][]) => void;
  };
};

export type AppState = Actions &
  (
    | ({ ready: false } & Base)
    | ({
        ready: true;
        wasm: WasmRuntime;
        replicaId: ReplicaId;
        squares: Record<SquareId, Square>;
      } & Base)
  );

export const useAppState = create<AppState>((set, get) => ({
  ready: false,
  squares: {},
  cursors: {},
  deltas() {
    const state = get();
    if (!state.ready) {
      return console.error("Not ready");
    }

    return state.wasm.deltas!();
  },
  setReady(wasm, replicaId) {
    if (get().ready) {
      console.error("Already ready");
      return;
    }

    // Run this to initialize the panic hook cause
    // I'm lazy
    wasm.get!();

    set({
      ready: true,
      wasm,
      replicaId,
    });
  },
  local: {
    setSquare(id, square) {
      const state = get();
      if (!state.ready) throw new Error("Not ready");

      state.wasm.set!(state.replicaId, id, square);

      set({
        squares: { ...state.squares, [id]: square },
      });
    },
  },
  remote: {
    mergeDeltas(deltas) {
      const state = get();
      if (!state.ready) throw new Error("Not ready");

      state.wasm.mergeDeltas!(deltas);
      const squares = state.wasm.get!();

      set({
        squares: awormapToRecord(squares),
      });
    },
    merge(remoteState) {
      const state = get();
      if (!state.ready) throw new Error("Not ready");

      const squares = state.wasm.merge!(remoteState);

      set({
        squares: awormapToRecord(squares),
      });
    },
    setCursors(cursors) {
      const state = get();
      if (state.ready !== true) return;
      const newCursors: AppState["cursors"] = cursors.reduce(
        (acc, [x, y, id]) => ({ ...acc, [id]: [x, y] }),
        {}
      );

      set({
        cursors: { ...state.cursors, ...newCursors },
      });
    },
  },
}));

const awormapToRecord = <K extends string | number | symbol, V>(
  map: AWORMap<K, V>
): Record<K, V> => {
  const record: Record<K, V> = {};
  for (const [_, val] of Object.entries(map.keys.kernel.entries)) {
    record[val[0]] = val[1];
  }
  return record;
};
