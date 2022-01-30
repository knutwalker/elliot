use std::{
    error::Error as StdError,
    fmt::{Debug, Display},
};

pub type BoxErr = Box<dyn StdError + Send + Sync + 'static>;

pub enum Error<T> {
    NoActorRef(NoActorRef),
    Stopped(Stopped),
    Unhandled(ActorRefGone<T>),
    Crashed(BoxErr),
}

#[derive(Copy, Clone, Debug)]
pub struct NoActorRef;

#[derive(Copy, Clone, Debug)]
pub struct Stopped;

#[derive(Clone)]
pub struct ActorRefGone<T>(pub T);

impl Display for NoActorRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad("No actor refs are alive, stopping actor")
    }
}

impl StdError for NoActorRef {}

impl Display for Stopped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad("Actor stopped on its own volition")
    }
}

impl StdError for Stopped {}

impl<T> Debug for ActorRefGone<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Unhandled").finish_non_exhaustive()
    }
}

impl<T> Display for ActorRefGone<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad("The recipient ActorRef is no longer available")
    }
}

impl<T> StdError for ActorRefGone<T> {}

impl<T> Debug for Error<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoActorRef(e) => Debug::fmt(e, f),
            Self::Stopped(e) => Debug::fmt(e, f),
            Self::Unhandled(e) => Debug::fmt(e, f),
            Self::Crashed(e) => f.debug_tuple("Crashed").field(e).finish(),
        }
    }
}

impl<T> Display for Error<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoActorRef(e) => Display::fmt(e, f),
            Error::Stopped(e) => Display::fmt(e, f),
            Error::Unhandled(e) => Display::fmt(e, f),
            Error::Crashed(e) => f.write_fmt(format_args!("Error while handling the message: {e}")),
        }
    }
}

impl<T> StdError for Error<T> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::Crashed(e) => Some(&**e),
            _ => None,
        }
    }
}
