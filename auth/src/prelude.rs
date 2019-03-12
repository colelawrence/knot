use crate::error;
use futures::future;
use std::result;

pub use crate::error::Error;
pub use failure::{Fail, ResultExt};

pub type Result<T, E = error::Error> = result::Result<T, E>;
pub type AppFuture<T, E = error::Error> = Box<future::Future<Item = T, Error = E>>;
