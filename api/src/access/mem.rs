///! Memory(redis) commands for AccessExecutor
use actix::prelude::*;
use actix_web::{error, Error};
use futures::future::Future;

use crate::mem::models::*;
use crate::mem::user_sessions;

use super::AccessExecutor;

/// Message returns new session key
pub use user_sessions::CreateSession;

impl Handler<CreateSession> for AccessExecutor {
    type Result = ResponseFuture<UserSession, Error>;

    fn handle(&mut self, msg: CreateSession, _: &mut Self::Context) -> Self::Result {
        Box::new(
            self.mem
                .send(msg)
                .map_err(error::ErrorInternalServerError)
                .flatten(),
        )
    }
}

pub use user_sessions::GetSessionByKey;

impl Handler<GetSessionByKey> for AccessExecutor {
    type Result = ResponseFuture<Option<UserSession>, Error>;

    fn handle(&mut self, msg: GetSessionByKey, _: &mut Self::Context) -> Self::Result {
        Box::new(
            self.mem
                .send(msg)
                .map_err(error::ErrorInternalServerError)
                .flatten(),
        )
    }
}

pub use user_sessions::DeleteSessionByKey;

impl Handler<DeleteSessionByKey> for AccessExecutor {
    type Result = ResponseFuture<(), Error>;

    fn handle(&mut self, msg: DeleteSessionByKey, _: &mut Self::Context) -> Self::Result {
        Box::new(
            self.mem
                .send(msg)
                .map_err(error::ErrorInternalServerError)
                .flatten(),
        )
    }
}

pub use user_sessions::{AddTokenToSession, AddTokenToSessionResult};

impl Handler<AddTokenToSession> for AccessExecutor {
    type Result = ResponseFuture<AddTokenToSessionResult, Error>;

    fn handle(&mut self, msg: AddTokenToSession, _: &mut Self::Context) -> Self::Result {
        Box::new(
            self.mem
                .send(msg)
                .map_err(error::ErrorInternalServerError)
                .flatten(),
        )
    }
}

pub use user_sessions::{AddUserToSession, AddUserToSessionResult};

impl Handler<AddUserToSession> for AccessExecutor {
    type Result = ResponseFuture<AddUserToSessionResult, Error>;

    fn handle(&mut self, msg: AddUserToSession, _: &mut Self::Context) -> Self::Result {
        Box::new(
            self.mem
                .send(msg)
                .map_err(error::ErrorInternalServerError)
                .flatten(),
        )
    }
}
