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
