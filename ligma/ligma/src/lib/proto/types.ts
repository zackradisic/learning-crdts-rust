// ============================================= //
// Types for WebAssembly runtime                 //
//                                               //
// This file is generated. PLEASE DO NOT MODIFY. //
// ============================================= //

export type AWORMap<K, V> = {
    keys: AWORSet<KeyVal<K, V>>;
};

export type AWORSet<V> = {
    kernel: DotKernel<V>;
    delta?: DotKernel<V>;
};

export type Dot = {
    : ReplicaId;
    : number;
};

export type DotCtx = {
    clock: VectorClock;
    dot_cloud: Array<Dot>;
};

export type DotKernel<V> = {
    ctx: DotCtx;
    entries: Record<Dot, V>;
};

/**
 * Key-value pair so it can implement Serializable, note that
 * it also implements PartialEq but only compares keys
 */
export type KeyVal<K, V> = {
    key: K;
    val: V;
};

export type ReplicaId = number;

export type Square = {
    x: number;
    y: number;
    width: number;
    height: number;
};

export type SquareId = number;

export type VectorClock = Record<ReplicaId, number>;
