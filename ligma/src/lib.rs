use std::{cell::RefCell, panic};

use ligma_protocol::*;
use once_cell::sync::Lazy;
use sypytkowski_blog::delta_state::awormap::Deltas;

// We maintain the global state in a mutable static so that we do not need to pass it from
// JavaScript every time we call the reducer. This avoids significant serialization overhead we
// would incur otherwise.
static mut STATE: Lazy<RefCell<AWORMap<SquareId, Square>>> =
    Lazy::new(|| RefCell::new(AWORMap::default()));

fn panic_hook() {
    fn hook_impl(info: &panic::PanicInfo) {
        let mut msg = info.to_string();

        // Add the error stack to our message.
        //
        // This ensures that even if the `console` implementation doesn't
        // include stacks for `console.error`, the stack is still available
        // for the user. Additionally, Firefox's console tries to clean up
        // stack traces, and ruins Rust symbols in the process
        // (https://bugzilla.mozilla.org/show_bug.cgi?id=1519569) but since
        // it only touches the logged message's associated stack, and not
        // the message's contents, by including the stack in the message
        // contents we make sure it is available to the user.
        // msg.push_str("\n\nStack:\n\n");
        // let e = Error::new();
        // let stack = e.stack();
        // msg.push_str(&stack);

        // Safari's devtools, on the other hand, _do_ mess with logged
        // messages' contents, so we attempt to break their heuristics for
        // doing that by appending some whitespace.
        // https://github.com/rustwasm/console_error_panic_hook/issues/7
        // msg.push_str("\n\n");

        // Finally, log the panic with `console.error`!
        log(msg);
    }
    panic::set_hook(Box::new(hook_impl))
}

#[fp_export_impl(ligma_protocol)]
fn get() -> AWORMap<SquareId, Square> {
    panic_hook();
    log("hello".to_string());
    unsafe { STATE.get_mut().clone() }
}

#[fp_export_impl(ligma_protocol)]
fn merge(delta: Deltas<SquareId, Square>) {
    log("HI".to_string());
    let state = unsafe { STATE.get_mut() };
    state.merge_delta(delta);
}

#[fp_export_impl(ligma_protocol)]
fn set(replica: sypytkowski_blog::ReplicaId, id: SquareId, square: Square) {
    let state = unsafe { STATE.get_mut() };
    state.insert(replica, id, square)
}

#[fp_export_impl(ligma_protocol)]
fn deltas() -> Deltas<SquareId, Square> {
    let state = unsafe { STATE.get_mut() };
    let deltas = state.split_mut();
    deltas.unwrap_or(Default::default())
}

#[fp_export_impl(ligma_protocol)]
fn replace(map: AWORMap<SquareId, Square>) {
    let state = unsafe { STATE.get_mut() };
    *state = map;
}
