use elliot::{ActorRef, ActorRefGone, ActorSystem};
use std::time::Instant;

struct Ping {
    count: usize,
    reply: ActorRef<Pong>,
}

struct Pong {
    count: usize,
    reply: ActorRef<Ping>,
}

/// receives pings and responds with pongs
async fn pong(this: ActorRef<Ping>, ping: Ping) -> Result<(), ActorRefGone<Pong>> {
    ping.reply.tell(Pong {
        count: ping.count,
        reply: this,
    })
}

/// receives a pong and sends a new ping or stops when count reached zero
async fn ping(this: ActorRef<Pong>, pong: Pong) -> Option<Result<(), ActorRefGone<Ping>>> {
    let count = pong.count.checked_sub(1)?;
    Some(pong.reply.tell(Ping { count, reply: this }))
}

async fn ping_pong(messages: usize) {
    let system = ActorSystem::new();

    let pinger = system.spawn("ping", ping);
    let ponger = system.spawn("pong", pong);

    let start = Instant::now();

    let _ = pinger.tell(Pong {
        count: messages,
        reply: ponger,
    });

    pinger.wait_for_stop().await;

    let took = start.elapsed();
    println!(
        "Sending {messages} ping-pongs in {took:?} for a throughput of {} msg/s",
        (messages * 2) as f64 / took.as_secs_f64()
    );
}

#[test]
fn test() {
    #[cfg(debug_assertions)]
    let messages = 10_000;

    #[cfg(not(debug_assertions))]
    let messages = 10_000_000;

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .build()
        .unwrap()
        .block_on(ping_pong(messages));
}
