///! Database commands for AccessExecutor
use actix::prelude::*;
use actix_web::{error, Error};
use futures::future::Future;

use crate::db::models::*;
use crate::db::user_tokens;

use super::AccessExecutor;

pub use user_tokens::UpsertUserToken;

impl Handler<UpsertUserToken> for AccessExecutor {
    type Result = ResponseFuture<(UserToken, Option<User>), Error>;

    fn handle(&mut self, msg: UpsertUserToken, _: &mut Self::Context) -> Self::Result {
        Box::new(
            self.db
                .send(msg)
                .map_err(error::ErrorInternalServerError)
                .and_then(|res| res),
        )
    }
}

pub use user_tokens::GetTokenForResourceId;

impl Handler<GetTokenForResourceId> for AccessExecutor {
    type Result = ResponseFuture<Option<UserToken>, Error>;

    fn handle(&mut self, msg: GetTokenForResourceId, _: &mut Self::Context) -> Self::Result {
        Box::new(
            self.db
                .send(msg)
                .map_err(error::ErrorInternalServerError)
                .and_then(|res| res),
        )
    }
}

pub use user_tokens::UpsertUserWithToken;

impl Handler<UpsertUserWithToken> for AccessExecutor {
    type Result = ResponseFuture<(UserToken, User), Error>;

    fn handle(&mut self, msg: UpsertUserWithToken, _: &mut Self::Context) -> Self::Result {
        Box::new(
            self.db
                .send(msg)
                .map_err(error::ErrorInternalServerError)
                .and_then(|res| res),
        )
    }
}

pub use user_tokens::UpdateUser;

impl Handler<UpdateUser> for AccessExecutor {
    type Result = ResponseFuture<User, Error>;

    fn handle(&mut self, msg: UpdateUser, _: &mut Self::Context) -> Self::Result {
        Box::new(
            self.db
                .send(msg)
                .map_err(error::ErrorInternalServerError)
                .and_then(|res| res),
        )
    }
}

pub use user_tokens::GetUserById;

impl Handler<GetUserById> for AccessExecutor {
    type Result = ResponseFuture<Option<User>, Error>;

    fn handle(&mut self, msg: GetUserById, _: &mut Self::Context) -> Self::Result {
        Box::new(
            self.db
                .send(msg)
                .map_err(error::ErrorInternalServerError)
                .and_then(|res| res),
        )
    }
}
