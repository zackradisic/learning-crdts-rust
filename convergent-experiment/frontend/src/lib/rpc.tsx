import { decode, encode } from "@msgpack/msgpack";
import { AWORMap, Deltas, ReplicaId, Square, SquareId } from "./proto/types";

export type ServerBound =
  | {
      type: "sync";
      replicaId: number;
      state: AWORMap<SquareId, Square>;
    }
  | {
      type: "update";
      deltas: Deltas<SquareId, Square>;
    }
  | {
      type: "cursor";
      pos: [x: number, y: number];
    };

export type ClientBound =
  | {
      type: "sync";
      state: AWORMap<SquareId, Square>;
    }
  | {
      type: "update";
      deltas: Deltas<SquareId, Square>;
    }
  | {
      type: "cursor";
      pos: [x: number, y: number, id: ReplicaId][];
    };

export const encodeServerBound = (msg: ServerBound): Uint8Array => encode(msg);
export const decodeClientBound = (
  msg: ArrayLike<number> | BufferSource
): ClientBound => decode(msg) as ClientBound;

export const prettyClientBound = (
  msg: ClientBound
): React.ReactNode | undefined => {
  switch (msg.type) {
    case "sync":
      return `Sync`;
    case "update":
      const entries: Record<string, [SquareId, Square]> = msg.deltas.entries;
      const str = Object.entries(entries)
        .map(([dot, [id, square]]) => `Dot = ${dot} Id = ${id}`)
        .join(", ");
      return str.length === 0 ? (
        "Delete"
      ) : (
        <>
          <b className="text-sm text-green-500">Update</b> {str}{" "}
          <button className="rounded bg-[#0c8ce9] p-1">
            Show version vector
          </button>
          <button className="ml-1 rounded bg-[#0c8ce9] p-1">Show deltas</button>
        </>
      );
  }
};
