use super::models;
use super::MemExecutor;
use crate::prelude::*;
use crate::utils::secure_rand_hex;
use futures::{
    future::{self, Either},
    Future,
};

use std::convert::From;

use crate::auth::{LoginAccessKey, UserAccessKey};

// 120 minutes
const SIGNUP_SESSION_EXPIRATION: std::time::Duration = std::time::Duration::from_secs(60 * 120);
// 10 minutes
const HANDOFF_EXPIRATION: std::time::Duration = std::time::Duration::from_secs(60 * 10);

pub struct HandoffState(pub String);

pub use models::IAm;

/// Create a state which is associated with this signup session
pub fn create_login_handoff(
    mem: &MemExecutor,
    login_access_key: &LoginAccessKey,
    redirect_uri: Option<&String>
) -> AppFuture<HandoffState> {
    let mem: MemExecutor = mem.clone();
    let redirect_uri = redirect_uri.cloned();
    Box::new(get_login_session(&mem, login_access_key).and_then(
        move |auth: models::LoginSession| {
            let signup_session_key = auth.key.clone();
            create_login_handoff_r(mem.clone(), signup_session_key, redirect_uri, 5).and_then(
                move |state_handoff| {
                    let state = state_handoff.key.clone();
                    mem.set_json(&auth, &SIGNUP_SESSION_EXPIRATION)
                        .map(move |_| HandoffState(state))
                },
            )
        },
    ))
}

fn create_login_handoff_r(
    mem: MemExecutor,
    session_key: String,
    redirect_uri: Option<String>,
    attempts_left: usize,
) -> AppFuture<models::StateHandoff> {
    let handoff = models::StateHandoff::login(&secure_rand_hex(12), &session_key, redirect_uri.as_ref());
    Box::new(
        mem.set_json_if_not_exists(&handoff, &HANDOFF_EXPIRATION)
            .from_err()
            .and_then(move |success_tf| {
                if success_tf {
                    Either::A(future::ok(handoff))
                } else if attempts_left <= 0 {
                    error!(
                        "create_login_handoff_r: Ran out of attempts to create a new handoff! Last tried: {}",
                        handoff.key,
                    );
                    Either::B(Either::A(future::err(Error::InternalServerError)))
                } else {
                    Either::B(Either::B(create_login_handoff_r(mem, session_key, redirect_uri, attempts_left - 1)))
                }
            }),
    )
}

/// On callback, assign identity information to the signup session to be used for completing signup
pub fn link_state_to_i_am(mem: &MemExecutor, state: String, i_am: models::IAm) -> AppFuture<LinkOutput> {
    let mem: MemExecutor = mem.clone();
    Box::new(
        mem.get_json::<models::StateHandoff>(&state)
            .and_then(|handoff_opt| {
                handoff_opt.ok_or(Error::BadRequest(String::from(
                    "Login state handoff does not exist.",
                )))
            })
            .and_then(move |handoff: models::StateHandoff| {
                let redirect_uri_opt = handoff.redirect_uri;
                get_login_session(&mem, &LoginAccessKey(handoff.session_key)).and_then(
                    move |mut login_session: models::LoginSession| {
                        login_session.i_am = Some(i_am);
                        mem.set_json(&login_session, &SIGNUP_SESSION_EXPIRATION)
                    },
                ).map(|_| LinkOutput {
                    redirect_uri_opt: redirect_uri_opt,
                })
            }),
    )
}

pub struct LinkOutput {
    pub redirect_uri_opt: Option<String>,
}

/// On callback, assign identity information to the signup session to be used for completing signup
pub fn link_state_to_user_id(mem: &MemExecutor, state: String, user_id: String) -> AppFuture<LinkOutput> {
    let mem: MemExecutor = mem.clone();
    Box::new(
        mem.get_json::<models::StateHandoff>(&state)
            .and_then(|handoff_opt| {
                handoff_opt.ok_or(Error::BadRequest(String::from(
                    "Login state handoff does not exist.",
                )))
            })
            .and_then(move |handoff: models::StateHandoff| {
                let redirect_uri_opt = handoff.redirect_uri;
                link_login_session_to_user_id(&mem, &LoginAccessKey(handoff.session_key), user_id).map(|_| LinkOutput {
                    redirect_uri_opt: redirect_uri_opt,
                })
            }),
    )
}

/// On callback, assign identity information to the signup session to be used for completing signup
pub fn link_login_session_to_user_id(
    mem: &MemExecutor,
    login: &LoginAccessKey,
    user_id: String,
) -> AppFuture<()> {
    let mem: MemExecutor = mem.clone();
    Box::new(get_login_session(&mem, login).and_then(
        move |mut login_session: models::LoginSession| {
            login_session.user_id = Some(user_id);
            mem.set_json(&login_session, &SIGNUP_SESSION_EXPIRATION)
        },
    ))
}

pub fn create_login_access_key(mem: &MemExecutor) -> AppFuture<LoginAccessKey> {
    Box::new(
        create_login_access_key_r(mem.clone(), 5).map(|signup_session: models::LoginSession| {
            LoginAccessKey(signup_session.key.to_string())
        }),
    )
}

fn create_login_access_key_r(
    mem: MemExecutor,
    attempts_left: usize,
) -> AppFuture<models::LoginSession> {
    let signup_session = models::LoginSession::from_key(secure_rand_hex(12));
    Box::new(
        mem.set_json_if_not_exists(&signup_session, &SIGNUP_SESSION_EXPIRATION)
            .from_err()
            .and_then(move |success_tf| {
                if success_tf {
                    Either::A(future::ok(signup_session))
                } else if attempts_left <= 0 {
                    error!(
                        "create_login_access_key_r: Ran out of attempts to create a new signup_session! Last tried: {:?}",
                        signup_session.key,
                    );
                    Either::B(Either::A(future::err(Error::InternalServerError)))
                } else {
                    Either::B(Either::B(create_login_access_key_r(mem, attempts_left - 1)))
                }
            }),
    )
}

use crate::db::models::User;

pub fn create_user_access_key(mem: &MemExecutor, user: User) -> AppFuture<UserAccessKey> {
    Box::new(
        create_user_access_key_r(mem.clone(), models::MemUser::from(user), 5)
            .map(|user_session: models::UserSession| UserAccessKey(user_session.key.to_string())),
    )
}

fn create_user_access_key_r(
    mem: MemExecutor,
    user: models::MemUser,
    attempts_left: usize,
) -> AppFuture<models::UserSession> {
    let user_session = models::UserSession::from_key_and_user(secure_rand_hex(12), user.clone());
    Box::new(
        mem.set_json_if_not_exists(&user_session, &SIGNUP_SESSION_EXPIRATION)
            .from_err()
            .and_then(move |success_tf| {
                if success_tf {
                    Either::A(future::ok(user_session))
                } else if attempts_left <= 0 {
                    error!(
                        "create_user_access_key_r: Ran out of attempts to create a new user_session! Last tried: {:?}",
                        user_session.key,
                    );
                    Either::B(Either::A(future::err(Error::InternalServerError)))
                } else {
                    Either::B(Either::B(create_user_access_key_r(mem, user, attempts_left - 1)))
                }
            }),
    )
}

fn get_login_session(
    mem: &MemExecutor,
    login_access_key: &LoginAccessKey,
) -> AppFuture<models::LoginSession> {
    Box::new(
        mem.get_json(&login_access_key.0)
            .and_then(|signup_session_opt| {
                signup_session_opt.ok_or(Error::BadRequest(String::from(
                    "Login session no longer exists.",
                )))
            }),
    )
}

pub fn get_login_session_opt(
    mem: &MemExecutor,
    login_access_key: &LoginAccessKey,
) -> impl Future<Item = Option<models::LoginSession>, Error = Error> {
    mem.get_json(&login_access_key.0)
}

pub fn get_user_session_opt(
    mem: &MemExecutor,
    user_access_key: &UserAccessKey,
) -> impl Future<Item = Option<models::UserSession>, Error = Error> {
    mem.get_json(&user_access_key.0)
}
