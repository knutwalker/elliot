use crate::{
    ActorContext, ActorRef, ActorRefGone, BoxErr, Error, NoActorRef, State, Stopped, SystemBus,
};
use pin_project_lite::pin_project;
use std::{
    error::Error as StdError,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::mpsc;

#[derive(Debug, Copy, Clone)]
pub enum Behaviors {
    /// A behavior that treats every incoming message as unhandled.
    Empty,
    /// A behavior that ignores every incoming message and returns `Same`.
    Ignore,
    /// Reuse the previous behavior while having handled the message.
    Same,
    /// Reuse the previous behavior while hinting that the message has not been handled.
    Unhandled,
    /// Stop accepting new messages voluntarily.
    Stopped,
}

pub(crate) fn actor_of<T: Send + 'static, N, A, Args>(name: N, behavior: A) -> ActorRef<T>
where
    N: Into<Arc<str>>,
    A: Behavior<T, Args>,
{
    let (tx, rx) = mpsc::unbounded_channel();
    let this = ActorRef { tx };
    let context = ActorContext::new(this.clone(), name.into());
    let _handle = tokio::spawn(async move { receive(context, rx, behavior).await });
    this
}

pub trait Behavior<T, Args = ()>: Send + Sync + Sized + 'static {
    type F: Future<Output = Result<Behaviors, Error<T>>> + Send;

    fn receive(&self, context: &ActorContext<T>, msg: T) -> Self::F;
}

pin_project! {
    pub struct MapErr<F, T> {
        #[pin]
        inner: F,
        _msg: PhantomData<T>,
    }
}

impl<F, Res, T> Future for MapErr<F, T>
where
    F: Future<Output = Res>,
    Res: IntoResult<T>,
{
    type Output = Result<Behaviors, Error<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.inner.poll(cx) {
            Poll::Ready(v) => Poll::Ready(v.into_result()),
            Poll::Pending => Poll::Pending,
        }
    }
}

async fn receive<B, T, Args>(
    context: ActorContext<T>,
    mut rx: mpsc::UnboundedReceiver<T>,
    behavior: B,
) -> Result<(), Error<T>>
where
    B: Behavior<T, Args>,
{
    loop {
        let msg = match receive_next(&mut rx).await {
            Some(msg) => msg,
            None => return Err(Error::NoActorRef(NoActorRef)),
        };
        let handled = behavior.receive(&context, msg).await;
        match handled {
            Ok(behavior) => match behavior {
                Behaviors::Empty => return empty_behavor(rx).await.map_err(Error::NoActorRef),
                Behaviors::Ignore => return ignore_behavor(rx).await.map_err(Error::NoActorRef),
                Behaviors::Same => {}
                Behaviors::Unhandled => {
                    // TODO: dead letters / unhandled bus
                }
                Behaviors::Stopped => {
                    drop(rx);
                    return Err(Error::Stopped(Stopped));
                }
            },
            Err(err) => return Err(err),
        }
    }
}

async fn receive_next<T>(rx: &mut mpsc::UnboundedReceiver<T>) -> Option<T> {
    let msg = rx.try_recv();
    match msg {
        Ok(msg) => Some(msg),
        Err(mpsc::error::TryRecvError::Disconnected) => None,
        Err(_) => rx.recv().await,
    }
}

async fn empty_behavor<T>(mut rx: mpsc::UnboundedReceiver<T>) -> Result<(), NoActorRef> {
    loop {
        let msg = match receive_next(&mut rx).await {
            Some(msg) => msg,
            None => return Err(NoActorRef),
        };
        // TODO: dead letters / unhandled bus
        drop(msg);
    }
}

async fn ignore_behavor<T>(mut rx: mpsc::UnboundedReceiver<T>) -> Result<(), NoActorRef> {
    loop {
        let msg = match receive_next(&mut rx).await {
            Some(msg) => msg,
            None => return Err(NoActorRef),
        };
        drop(msg);
    }
}

pub trait FromContext<T> {
    fn from_context(context: &ActorContext<T>) -> Self;
}

impl<T> FromContext<T> for ActorContext<T> {
    fn from_context(context: &ActorContext<T>) -> Self {
        Clone::clone(context)
    }
}

impl<T> FromContext<T> for ActorRef<T> {
    fn from_context(context: &ActorContext<T>) -> Self {
        context.this()
    }
}

impl<T, S: Default> FromContext<T> for State<S> {
    fn from_context(_context: &ActorContext<T>) -> Self {
        Self(Default::default())
    }
}

impl<T, E> FromContext<T> for SystemBus<E> {
    fn from_context(_context: &ActorContext<T>) -> Self {
        Self(None)
    }
}

pub trait IntoResult<T> {
    fn into_result(self) -> Result<Behaviors, Error<T>>;
}

impl<T> IntoResult<T> for Behaviors {
    fn into_result(self) -> Result<Behaviors, Error<T>> {
        Ok(self)
    }
}

impl<T> IntoResult<T> for () {
    fn into_result(self) -> Result<Behaviors, Error<T>> {
        Ok(Behaviors::Same)
    }
}

impl<T, O, E> IntoResult<T> for Result<O, E>
where
    T: 'static,
    O: IntoResult<T>,
    E: StdError + Send + Sync + 'static,
{
    fn into_result(self) -> Result<Behaviors, Error<T>> {
        match self {
            Ok(ok) => ok.into_result(),
            Err(err) => {
                let err: BoxErr = Box::new(err);
                let err = match err.downcast::<ActorRefGone<T>>() {
                    Ok(unhandled) => Error::Unhandled(*unhandled),
                    Err(err) => Error::Crashed(err),
                };
                Err(err)
            }
        }
    }
}

impl<T, O: IntoResult<T>> IntoResult<T> for Option<O> {
    fn into_result(self) -> Result<Behaviors, Error<T>> {
        match self {
            Some(ok) => ok.into_result(),
            None => Ok(Behaviors::Stopped),
        }
    }
}

macro_rules! impl_behavior {
    ( $($ty:ident),* $(,)? ) => {
        impl<F, Fut, Res, T, $($ty,)*> $crate::Behavior<T, ($($ty,)*)> for F
        where
            F: ::std::ops::Fn($($ty,)* T) -> Fut + ::std::marker::Send + Sync + 'static,
            Fut: ::std::future::Future<Output = Res> + ::std::marker::Send,
            Res: $crate::behavior::IntoResult<T>,
            T: ::std::marker::Send + 'static,
            $( $ty: $crate::behavior::FromContext<T> + ::std::marker::Send,)*
        {
            type F = $crate::behavior::MapErr<Fut, T>;

            fn receive(&self, _context: &ActorContext<T>, msg: T) -> Self::F {
                let f = (self)(
                    $($ty::from_context(_context),)*
                    msg,
                );
                MapErr { inner: f, _msg: ::std::marker::PhantomData }
            }
        }
    };
}

macro_rules! impl_for_tuples {
    ($name:ident) => {
        $name!();
        $name!(T1);
        $name!(T1, T2);
        $name!(T1, T2, T3);
        $name!(T1, T2, T3, T4);
        $name!(T1, T2, T3, T4, T5);
        $name!(T1, T2, T3, T4, T5, T6);
        $name!(T1, T2, T3, T4, T5, T6, T7);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);
    };
}

impl_for_tuples!(impl_behavior);
