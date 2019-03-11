use actix::prelude::*;
use actix_redis::RedisActor;
use actix_web::{Error, FutureResponse, Result};

use futures::future::{self, Future};

use super::util;

use super::{get_set, models, MemExecutor};

/// How much time between initiating login and handoff before expiring?
const LOGIN_HANDOFF_EXPIRE_IN_SECS: u64 = 60 * 10;
/// How often should we require a new session?
const NEW_SESSIONS_EXPIRE_IN_SECS: u64 = 60 * 60;
const SESSIONS_WITH_TOKENS_EXPIRE_IN_SECS: u64 = 60 * 60 * 24 * 5;
const SESSIONS_WITH_USER_EXPIRE_IN_SECS: u64 = 60 * 60 * 24 * 365;

const KEY_PREFIX_SESSION: &'static str = "s";
const KEY_PREFIX_HANDOFF: &'static str = "h";

fn get_session_by_key(
    conn: &Addr<RedisActor>,
    by_key: &str,
) -> FutureResponse<Option<models::UserSession>> {
    get_set::get_json(conn, KEY_PREFIX_SESSION, by_key)
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
    get_set::set_json(conn, KEY_PREFIX_SESSION, by_key, value, &expires_in)
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
        get_set::set_json_if_not_exists(
            &conn,
            KEY_PREFIX_SESSION,
            &new_session.key,
            &new_session,
            &expires_in,
        )
        .and_then(move |success_tf| {
            if success_tf {
                future::Either::A(future::ok(new_session))
            } else {
                error!(
                    "create_session: New session key collision on {}",
                    new_session.key
                );
                future::Either::B(create_session(&conn))
            }
        }),
    )
}

fn delete_session_by_key(conn: &Addr<RedisActor>, by_key: &str) -> FutureResponse<()> {
    get_set::delete(conn, KEY_PREFIX_SESSION, by_key)
}

/// Get and delete handoff value
fn take_session_key_by_handoff(
    conn: &Addr<RedisActor>,
    by_handoff: &str,
) -> FutureResponse<Option<String>> {
    let conn = conn.clone();
    let by_handoff = by_handoff.to_string();
    Box::new(
        get_set::get_json(&conn, KEY_PREFIX_HANDOFF, &by_handoff).and_then(
            move |session_key_opt| match session_key_opt {
                Some(session_key) => future::Either::A(
                    get_set::delete(&conn, KEY_PREFIX_HANDOFF, &by_handoff)
                        .map(move |_| Some(session_key)),
                ),
                None => future::Either::B(future::ok(None)),
            },
        ),
    )
}

/// Create a login handoff state
fn create_handoff_for_session_key_r(
    conn: Addr<RedisActor>,
    session_key: String,
    expires_in: std::time::Duration,
) -> FutureResponse<String> {
    let new_handoff = util::secure_rand_hex(16);
    Box::new(
        get_set::set_json_if_not_exists(
            &conn,
            KEY_PREFIX_HANDOFF,
            &new_handoff,
            &session_key,
            &expires_in,
        )
        .and_then(move |success_tf| {
            if success_tf {
                future::Either::A(future::ok(new_handoff))
            } else {
                error!(
                    "create_handoff_for_session_key_r: New handoff key collision on {}",
                    new_handoff
                );
                future::Either::B(create_handoff_for_session_key_r(
                    conn,
                    session_key,
                    expires_in,
                ))
            }
        }),
    )
}

/// Create a login handoff state
fn create_handoff_for_session_key(
    conn: &Addr<RedisActor>,
    session_key: &str,
) -> FutureResponse<String> {
    use std::time::Duration;
    let conn = conn.clone();
    let session_key = session_key.to_string();
    let new_handoff = util::secure_rand_hex(16);
    let expires_in = Duration::from_secs(LOGIN_HANDOFF_EXPIRE_IN_SECS);
    Box::new(
        get_session_by_key(&conn, &session_key).and_then(move |sess_opt| {
            if sess_opt.is_some() {
                future::Either::A(create_handoff_for_session_key_r(
                    conn,
                    session_key,
                    expires_in,
                ))
            } else {
                error!(
                    "create_handoff_for_session_key: Session key not found! {}",
                    session_key
                );
                future::Either::B(future::err(actix_web::error::ErrorBadRequest(
                    "Session key not found",
                )))
            }
        }),
    )
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

pub struct DeleteSessionByKey(pub String);

impl Message for DeleteSessionByKey {
    type Result = Result<()>;
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

impl Handler<DeleteSessionByKey> for MemExecutor {
    type Result = ResponseFuture<(), Error>;

    fn handle(&mut self, msg: DeleteSessionByKey, _: &mut Self::Context) -> Self::Result {
        delete_session_by_key(self.conn(), &msg.0)
    }
}

/// Create a temporary pointer to a session key for use in login states
pub struct CreateHandoffForSessionKey(pub String);

impl Message for CreateHandoffForSessionKey {
    type Result = Result<String>;
}

impl Handler<CreateHandoffForSessionKey> for MemExecutor {
    type Result = ResponseFuture<String, Error>;

    fn handle(&mut self, msg: CreateHandoffForSessionKey, _: &mut Self::Context) -> Self::Result {
        let conn = self.conn();
        create_handoff_for_session_key(&conn, &msg.0)
    }
}

/// Get and delete handoff value
pub struct TakeSessionKeyByHandoff(pub String);

impl Message for TakeSessionKeyByHandoff {
    type Result = Result<Option<String>>;
}

impl Handler<TakeSessionKeyByHandoff> for MemExecutor {
    type Result = ResponseFuture<Option<String>, Error>;

    fn handle(&mut self, msg: TakeSessionKeyByHandoff, _: &mut Self::Context) -> Self::Result {
        take_session_key_by_handoff(self.conn(), &msg.0)
    }
}
