# Learning CRDTs

Monorepo of CRDT implementations I write as I learn more about them

## Directory structure

### [sypytkowski-blog](/sypytkowski-blog)

Following along and porting the code from Bartosz Sypytkowski's [blog post series](https://bartoszsypytkowski.com/optimizing-state-based-crdts-1/) in F# to Rust

### convergent-experiment\*

An experiment in building a simple app with convergent/state-based CRDTs taught in the first
half of Bartosz Sypytkowski's blog post series

It is a web app where users can create and move around squares on a canvas that gets synced between multiple users through a websocket server.

The CRDTs from the [sypytkowski-blog](/sypytkowski-blog) crate are compiled to Wasm and bindings for TS are generated with [fp-bindgen](https://github.com/fiberplane/fp-bindgen)

```bash
# Generate the bindings for wasm
cargo run --package convergent-experiment-protocol-gen --bin convergent-experiment-protocol-gen

# Build wasm
cargo build --package convergent-experiment --target wasm32-unknown-unknown --release

# Copy wasm to frontend public folder
cp target/wasm32-unknown-unknown/release/convergent-experiment.wasm convergent-experiment/frontend/public/convergent-experiment.wasm

# Run the WS server
cargo run --package convergent-experiment-ws
```
