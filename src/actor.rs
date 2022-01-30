use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::ActorRefGone;
use tokio::sync::mpsc;

pub struct ActorRef<T> {
    pub(crate) tx: mpsc::UnboundedSender<T>,
}

impl<T> ActorRef<T> {
    pub fn tell(&self, msg: T) -> Result<(), ActorRefGone<T>> {
        if let Err(e) = self.tx.send(msg) {
            return Err(ActorRefGone(e.0));
        }
        Ok(())
    }

    pub fn is_alive(&self) -> bool {
        self.tx.is_closed() == false
    }

    pub async fn wait_for_stop(&self) {
        self.tx.closed().await
    }
}

#[derive(Debug)]
pub struct ActorContext<T> {
    // TODO weak, and ref = Arc(channel)
    this: ActorRef<T>,
    // TODO: add handle somehow
    name: Arc<str>,
}

impl<T> ActorContext<T> {
    pub(crate) fn new(this: ActorRef<T>, name: Arc<str>) -> Self {
        Self { this, name }
    }

    pub fn this(&self) -> ActorRef<T> {
        self.this.clone()
    }

    pub fn name(&self) -> &str {
        &*self.name
    }
}

impl<T> Clone for ActorRef<T> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<T> Clone for ActorContext<T> {
    fn clone(&self) -> Self {
        Self {
            this: self.this.clone(),
            name: Arc::clone(&self.name),
        }
    }
}

impl<T> std::fmt::Debug for ActorRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorRef").finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct SystemBus<T>(pub(crate) Option<T>);

impl<T> SystemBus<T> {
    pub async fn publish(&self, _msg: T) {}
}

#[derive(Debug)]
pub struct State<T>(pub(crate) T);

impl<T> Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for State<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
