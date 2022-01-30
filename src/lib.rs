/*!
Yet another actor library for rust.

# Work In Progress

This library is under development and should not be used.

# Motivation

Main difference between other actor offerings:

- no derive macros or trait impls required
- async actor message handler without having to use async_trait
- messages are plain rust structs without having to implement a trait
- actors can signal if they want to change their behavior

# Example

A ping-pong interaction that stops after one roundtrip

```rust
// Import library prelude
use elliot::*;

/// Message to send to the pong actor, contains the return address of the sender
struct Ping {
    count: usize,
    reply: ActorRef<Pong>,
}

/// Message to send to the ping actor
struct Pong {
    count: usize,
    reply: ActorRef<Ping>,
}

/// pong actor: receives pings and responds with pongs
/// We need access to the name and ref of the actor, so we require ActorContext as a parameter
/// Returning a result inidicates the error condition when the actor will stop
/// Otherwise, returning () keeps the actor alive (could also use `Behaviors::Same`)
async fn pong(ctx: ActorContext<Ping>, ping: Ping) -> Result<(), ActorRefGone<Pong>> {
    // should not really use blocking operations inside an actor
    println!("{} received a ping", ctx.name());
    ping.reply.tell(Pong {
        count: ping.count,
        reply: ctx.this(),
    })
}

/// ping actor: sends a ping and waits for a reply
/// We don't need the full context and can require the own ActorRef directly
/// Returning None signals that we want to stop without an error (could also use `Behaviors::Stopped`)
async fn ping(this: ActorRef<Pong>, pong: Pong) -> Option<Result<(), ActorRefGone<Ping>>> {
    let count = pong.count.checked_sub(1)?;
    Some(pong.reply.tell(Ping { count, reply: this }))
}

// a test scenario
// We can create actors from an actor system
async fn ping_pong() {
    let system = ActorSystem::new();

    let pinger = system.spawn("ping", ping);
    let ponger = system.spawn("pong", pong);

    // The initial message to the pinger with 3 iterations
    let _ = pinger.tell(Pong {
        count: 3,
        reply: ponger,
    });

    // wait until the pinger has received the pong and has stopped
    pinger.wait_for_stop().await;
}

// or use #[tokio::main]
fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .build()
        .unwrap()
        .block_on(ping_pong());
}
```
*/

#![warn(
    bad_style,
    const_err,
    dead_code,
    explicit_outlives_requirements,
    improper_ctypes,
    keyword_idents,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_copy_implementations,
    missing_debug_implementations,
    // missing_docs,
    no_mangle_generic_items,
    non_ascii_idents,
    non_shorthand_field_patterns,
    noop_method_call,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_in_public,
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unconditional_recursion,
    unreachable_pub,
    unsafe_code,
    unused_allocation,
    unused_comparisons,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_features,
    unused_import_braces,
    unused_lifetimes,
    unused_parens,
    unused_qualifications,
    unused_results,
    unused,
    variant_size_differences,
    while_true
)]
// #![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(
    clippy::bool_comparison,
    clippy::module_name_repetitions,
    clippy::redundant_pub_crate
)]

mod actor;
mod behavior;
mod error;

pub use actor::*;
pub use behavior::{Behavior, Behaviors};
pub use error::*;

#[allow(missing_copy_implementations)]
#[derive(Debug)]
pub struct ActorSystem {
    // TODO: dead letters, system bus, actor paths,
}

impl ActorSystem {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {}
    }

    pub fn spawn<T: Send + 'static, N, A, Args>(&self, name: N, behavior: A) -> ActorRef<T>
    where
        N: Into<std::sync::Arc<str>>,
        A: Behavior<T, Args>,
    {
        behavior::actor_of(name, behavior)
    }
}
