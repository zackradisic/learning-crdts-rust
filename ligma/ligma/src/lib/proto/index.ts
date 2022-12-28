// ============================================= //
// WebAssembly runtime for TypeScript            //
//                                               //
// This file is generated. PLEASE DO NOT MODIFY. //
// ============================================= //
// deno-lint-ignore-file no-explicit-any no-unused-vars

import { encode, decode } from "@msgpack/msgpack";

import type * as types from "./types";

type FatPtr = bigint;

export type Imports = {
    log: (str: string) => void;
};

export type Exports = {
    deltas?: () => Deltas<SquareId, Square>;
    get?: () => types.AWORMap<types.SquareId, types.Square>;
    merge?: (other: types.AWORMap<types.SquareId, types.Square>) => types.AWORMap<types.SquareId, types.Square>;
    mergeDeltas?: (delta: Deltas<SquareId, Square>) => void;
    replace?: (map: types.AWORMap<types.SquareId, types.Square>) => void;
    set?: (replica: types.ReplicaId, id: types.SquareId, square: types.Square) => void;
};

/**
 * Represents an unrecoverable error in the FP runtime.
 *
 * After this, your only recourse is to create a new runtime, probably with a different WASM plugin.
 */
export class FPRuntimeError extends Error {
    constructor(message: string) {
        super(message);
    }
}

/**
 * Creates a runtime for executing the given plugin.
 *
 * @param plugin The raw WASM plugin.
 * @param importFunctions The host functions that may be imported by the plugin.
 * @returns The functions that may be exported by the plugin.
 */
export async function createRuntime(
    plugin: ArrayBuffer,
    importFunctions: Imports
): Promise<Exports> {
    const promises = new Map<FatPtr, ((result: FatPtr) => void) | FatPtr>();

    function createAsyncValue(): FatPtr {
        const len = 12; // std::mem::size_of::<AsyncValue>()
        const fatPtr = malloc(len);
        const [ptr] = fromFatPtr(fatPtr);
        const buffer = new Uint8Array(memory.buffer, ptr, len);
        buffer.fill(0);
        return fatPtr;
    }

    function interpretSign(num: number, cap: number) {
        if (num < cap) {
            return num;
        } else {
            return num - (cap << 1);
        }
    }

    function interpretBigSign(num: bigint, cap: bigint) {
        if (num < cap) {
            return num;
        } else {
            return num - (cap << 1n);
        }
    }

    function parseObject<T>(fatPtr: FatPtr): T {
        const [ptr, len] = fromFatPtr(fatPtr);
        const buffer = new Uint8Array(memory.buffer, ptr, len);
        // Without creating a copy of the memory, we risk corruption of any
        // embedded `Uint8Array` objects returned from `decode()` after `free()`
        // has been called :(
        const copy = new Uint8Array(len);
        copy.set(buffer);
        free(fatPtr);
        const object = decode(copy) as unknown as T;
        return object;
    }

    function promiseFromPtr(ptr: FatPtr): Promise<FatPtr> {
        const resultPtr = promises.get(ptr);
        if (resultPtr) {
            if (typeof resultPtr === "function") {
                throw new FPRuntimeError("Already created promise for this value");
            }

            promises.delete(ptr);
            return Promise.resolve(resultPtr);
        } else {
            return new Promise((resolve) => {
                promises.set(ptr, resolve as (result: FatPtr) => void);
            });
        }
    }

    function resolvePromise(asyncValuePtr: FatPtr, resultPtr: FatPtr) {
        const resolve = promises.get(asyncValuePtr);
        if (resolve) {
            if (typeof resolve !== "function") {
                throw new FPRuntimeError("Tried to resolve invalid promise");
            }

            promises.delete(asyncValuePtr);
            resolve(resultPtr);
        } else {
            promises.set(asyncValuePtr, resultPtr);
        }
    }

    function serializeObject<T>(object: T): FatPtr {
        return exportToMemory(encode(object));
    }

    function exportToMemory(serialized: Uint8Array): FatPtr {
        const fatPtr = malloc(serialized.length);
        const [ptr, len] = fromFatPtr(fatPtr);
        const buffer = new Uint8Array(memory.buffer, ptr, len);
        buffer.set(serialized);
        return fatPtr;
    }

    function importFromMemory(fatPtr: FatPtr): Uint8Array {
        const [ptr, len] = fromFatPtr(fatPtr);
        const buffer = new Uint8Array(memory.buffer, ptr, len);
        const copy = new Uint8Array(len);
        copy.set(buffer);
        free(fatPtr);
        return copy;
    }

    const { instance } = await WebAssembly.instantiate(plugin, {
        fp: {
            __fp_gen_log: (str_ptr: FatPtr) => {
                const str = parseObject<string>(str_ptr);
                importFunctions.log(str);
            },
        },
    });

    const getExport = <T>(name: string): T => {
        const exp = instance.exports[name];
        if (!exp) {
            throw new FPRuntimeError(`Plugin did not export expected symbol: "${name}"`);
        }
        return exp as unknown as T;
    };

    const memory = getExport<WebAssembly.Memory>("memory");
    const malloc = getExport<(len: number) => FatPtr>("__fp_malloc");
    const free = getExport<(ptr: FatPtr) => void>("__fp_free");

    return {
        deltas: (() => {
            const export_fn = instance.exports.__fp_gen_deltas as any;
            if (!export_fn) return;

            return () => parseObject<Deltas<SquareId, Square>>(export_fn());
        })(),
        get: (() => {
            const export_fn = instance.exports.__fp_gen_get as any;
            if (!export_fn) return;

            return () => parseObject<types.AWORMap<types.SquareId, types.Square>>(export_fn());
        })(),
        merge: (() => {
            const export_fn = instance.exports.__fp_gen_merge as any;
            if (!export_fn) return;

            return (other: types.AWORMap<types.SquareId, types.Square>) => {
                const other_ptr = serializeObject(other);
                return parseObject<types.AWORMap<types.SquareId, types.Square>>(export_fn(other_ptr));
            };
        })(),
        mergeDeltas: (() => {
            const export_fn = instance.exports.__fp_gen_merge_deltas as any;
            if (!export_fn) return;

            return (delta: Deltas<SquareId, Square>) => {
                const delta_ptr = serializeObject(delta);
                export_fn(delta_ptr);
            };
        })(),
        replace: (() => {
            const export_fn = instance.exports.__fp_gen_replace as any;
            if (!export_fn) return;

            return (map: types.AWORMap<types.SquareId, types.Square>) => {
                const map_ptr = serializeObject(map);
                export_fn(map_ptr);
            };
        })(),
        set: (() => {
            const export_fn = instance.exports.__fp_gen_set as any;
            if (!export_fn) return;

            return (replica: types.ReplicaId, id: types.SquareId, square: types.Square) => {
                const replica_ptr = serializeObject(replica);
                const id_ptr = serializeObject(id);
                const square_ptr = serializeObject(square);
                export_fn(replica_ptr, id_ptr, square_ptr);
            };
        })(),
    };
}

function fromFatPtr(fatPtr: FatPtr): [ptr: number, len: number] {
    return [
        Number.parseInt((fatPtr >> 32n).toString()),
        Number.parseInt((fatPtr & 0xffff_ffffn).toString()),
    ];
}

function toFatPtr(ptr: number, len: number): FatPtr {
    return (BigInt(ptr) << 32n) | BigInt(len);
}
