use actix::prelude::*;
use actix_redis::RedisActor;
use actix_web::{Error, FutureResponse, Result};

use futures::future::{self, Future};

use super::util;

use super::{get_set, models, MemExecutor};

/// How often should we require a new session?
const NEW_SESSIONS_EXPIRE_IN_SECS: u64 = 60 * 60;
const SESSIONS_WITH_TOKENS_EXPIRE_IN_SECS: u64 = 60 * 60 * 24 * 5;
const SESSIONS_WITH_USER_EXPIRE_IN_SECS: u64 = 60 * 60 * 24 * 365;

fn get_session_by_key(
    conn: &Addr<RedisActor>,
    by_key: &str,
) -> FutureResponse<Option<models::UserSession>> {
    get_set::get_json(conn, "s", by_key)
}

fn set_session_by_key(
    conn: &Addr<RedisActor>,
    by_key: &str,
    value: &models::UserSession,
) -> FutureResponse<()> {
    use std::time::Duration;
    let expires_in = if value.user_id.is_some() {
        Duration::from_secs(SESSIONS_WITH_USER_EXPIRE_IN_SECS)
    } else if value.user_token_resource_id.is_some() {
        Duration::from_secs(SESSIONS_WITH_TOKENS_EXPIRE_IN_SECS)
    } else {
        Duration::from_secs(NEW_SESSIONS_EXPIRE_IN_SECS)
    };
    get_set::set_json(conn, "s", by_key, value, &expires_in)
}

fn create_session(conn: &Addr<RedisActor>) -> FutureResponse<models::UserSession> {
    use std::time::Duration;
    let conn = conn.clone();
    let new_session = models::UserSession {
        key: util::secure_rand_hex(16),
        user_token_resource_id: None,
        user_id: None,
    };
    let expires_in = Duration::from_secs(NEW_SESSIONS_EXPIRE_IN_SECS);
    Box::new(
        get_set::set_json_if_not_exists(&conn, "s", &new_session.key, &new_session, &expires_in)
            .and_then(move |success_tf| {
                if success_tf {
                    future::Either::A(future::ok(new_session))
                } else {
                    error!(
                        "create_session: New session key collision on {}",
                        new_session.key
                    );
                    let conn = conn;
                    future::Either::B(create_session(&conn))
                }
            }),
    )
}

fn delete_session_by_key(conn: &Addr<RedisActor>, by_key: &str) -> FutureResponse<()> {
    get_set::delete(conn, "s", by_key)
}

pub struct CreateSession();

impl Message for CreateSession {
    type Result = Result<models::UserSession>;
}

pub enum AddTokenToSessionResult {
    SessionNotFound,
    Success(models::UserSession),
}

pub struct AddTokenToSession {
    pub session_key: String,
    pub resource_id: String,
}

impl Message for AddTokenToSession {
    type Result = Result<AddTokenToSessionResult>;
}

pub enum AddUserToSessionResult {
    SessionNotFound,
    Success(models::UserSession),
}

pub struct AddUserToSession {
    pub session_key: String,
    pub user_id: String,
}

impl Message for AddUserToSession {
    type Result = Result<AddUserToSessionResult>;
}

pub struct GetSessionByKey(pub String);

impl Message for GetSessionByKey {
    type Result = Result<Option<models::UserSession>>;
}

impl Handler<CreateSession> for MemExecutor {
    type Result = ResponseFuture<models::UserSession, Error>;

    fn handle(&mut self, _: CreateSession, _: &mut Self::Context) -> Self::Result {
        let conn = self.conn();
        create_session(&conn)
    }
}

impl Handler<AddTokenToSession> for MemExecutor {
    type Result = ResponseFuture<AddTokenToSessionResult, Error>;

    fn handle(&mut self, msg: AddTokenToSession, _: &mut Self::Context) -> Self::Result {
        let conn = self.conn().clone();
        Box::new(
            get_session_by_key(&conn, &msg.session_key).and_then(move |session_opt| {
                match session_opt {
                    Some(mut session) => future::Either::A(
                        if session.user_token_resource_id.as_ref() == Some(&msg.resource_id) {
                            future::Either::A(future::ok(AddTokenToSessionResult::Success(session)))
                        } else {
                            session.user_token_resource_id = Some(msg.resource_id);
                            future::Either::B(
                                set_session_by_key(&conn, &session.key, &session)
                                    .map(|_| AddTokenToSessionResult::Success(session)),
                            )
                        },
                    ),
                    None => future::Either::B(future::ok(AddTokenToSessionResult::SessionNotFound)),
                }
            }),
        )
    }
}

impl Handler<AddUserToSession> for MemExecutor {
    type Result = ResponseFuture<AddUserToSessionResult, Error>;

    fn handle(&mut self, msg: AddUserToSession, _: &mut Self::Context) -> Self::Result {
        let conn = self.conn().clone();
        Box::new(
            get_session_by_key(&conn, &msg.session_key).and_then(move |session_opt| {
                match session_opt {
                    Some(mut session) => {
                        session.user_id = Some(msg.user_id);
                        future::Either::A(
                            set_session_by_key(&conn, &session.key, &session)
                                .map(|_| AddUserToSessionResult::Success(session)),
                        )
                    }
                    None => future::Either::B(future::ok(AddUserToSessionResult::SessionNotFound)),
                }
            }),
        )
    }
}

impl Handler<GetSessionByKey> for MemExecutor {
    type Result = ResponseFuture<Option<models::UserSession>, Error>;

    fn handle(&mut self, msg: GetSessionByKey, _: &mut Self::Context) -> Self::Result {
        get_session_by_key(self.conn(), &msg.0)
    }
}
