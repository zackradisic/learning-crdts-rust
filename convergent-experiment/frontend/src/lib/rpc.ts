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
