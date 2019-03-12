use super::models;
use super::MemExecutor;
use crate::prelude::*;
use crate::utils::secure_rand_hex;
use futures::{
    future::{self, Either},
    Future,
};

use crate::auth::{LoginAccessToken, UserAccessToken};

// 120 minutes
const SIGNUP_SESSION_EXPIRATION: std::time::Duration = std::time::Duration::from_secs(60 * 120);
// 10 minutes
const HANDOFF_EXPIRATION: std::time::Duration = std::time::Duration::from_secs(60 * 10);

pub struct HandoffState(pub String);

pub use models::IAm;

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum CredentialsJson {
    Login { access_token: LoginAccessToken },
    User { access_token: UserAccessToken },
}

/// Create a state which is associated with this signup session
pub fn create_login_handoff(
    mem: &MemExecutor,
    login_access_token: &LoginAccessToken,
) -> AppFuture<HandoffState> {
    let mem: MemExecutor = mem.clone();
    Box::new(get_login_session(&mem, login_access_token).and_then(
        move |auth: models::LoginSession| {
            let signup_session_key = auth.key.clone();
            create_login_handoff_r(mem.clone(), signup_session_key, 5).and_then(
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
    session_key: LoginAccessToken,
    attempts_left: usize,
) -> AppFuture<models::StateHandoff> {
    let handoff = models::StateHandoff::signup(&secure_rand_hex(12), &session_key);
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
                    Either::B(Either::B(create_login_handoff_r(mem, session_key, attempts_left - 1)))
                }
            }),
    )
}

/// On callback, assign identity information to the signup session to be used for completing signup
pub fn iam_callback(mem: &MemExecutor, state: String, i_am: models::IAm) -> AppFuture<()> {
    let mem: MemExecutor = mem.clone();
    Box::new(
        mem.get_json::<models::StateHandoff>(&state)
            .and_then(|handoff_opt| {
                handoff_opt.ok_or(Error::BadRequest(String::from(
                    "Login state handoff does not exist.",
                )))
            })
            .and_then(move |handoff: models::StateHandoff| {
                get_login_session(&mem, &handoff.session_key).and_then(
                    move |mut login_session: models::LoginSession| {
                        login_session.i_am = Some(i_am);
                        mem.set_json(&login_session, &SIGNUP_SESSION_EXPIRATION)
                    },
                )
            }),
    )
}

pub fn create_login_access_token(mem: &MemExecutor) -> AppFuture<LoginAccessToken> {
    Box::new(create_login_access_token_r(mem.clone(), 5).map(
        |signup_session: models::LoginSession| LoginAccessToken(signup_session.key.0.to_string()),
    ))
}

fn create_login_access_token_r(
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
                        "create_login_access_token_r: Ran out of attempts to create a new signup_session! Last tried: {:?}",
                        signup_session.key,
                    );
                    Either::B(Either::A(future::err(Error::InternalServerError)))
                } else {
                    Either::B(Either::B(create_login_access_token_r(mem, attempts_left - 1)))
                }
            }),
    )
}

fn get_login_session(
    mem: &MemExecutor,
    login_access_token: &LoginAccessToken,
) -> AppFuture<models::LoginSession> {
    Box::new(
        mem.get_json(&login_access_token.0)
            .and_then(|signup_session_opt| {
                signup_session_opt.ok_or(Error::BadRequest(String::from(
                    "Login session no longer exists.",
                )))
            }),
    )
}

pub fn get_login_session_opt(
    mem: &MemExecutor,
    login_access_token: &LoginAccessToken,
) -> impl Future<Item = Option<models::LoginSession>, Error = Error> {
    mem.get_json(&login_access_token.0)
}

pub fn get_user_session_opt(
    mem: &MemExecutor,
    user_access_token: &UserAccessToken,
) -> impl Future<Item = Option<models::UserSession>, Error = Error> {
    mem.get_json(&user_access_token.0)
}
