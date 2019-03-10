///! Google Oauth2 commands for AccessExecutor
use actix::prelude::*;
use actix_web::{error, Error, Result};
use futures::future::{self, Either, Future};

use super::clients::{google_oauth_client, google_people_client, GoogleAccessToken};
use crate::db::models::*;
use crate::db::user_tokens;
use crate::mem::models::*;
use crate::mem::user_sessions;

use self::google_oauth_client::ExchangeResult;

use super::AccessExecutor;

/// When a person gets their OAuth2 callback at this endpoint
/// This should resolve with an updated [UserSession]
pub struct GoogleOAuth2Callback {
    /// Maps to session key
    pub state: String,
    pub code: String,
    pub redirect_uri: String,
}

pub enum GoogleOAuth2CallbackErr {
    RevokedPreviousTokens,
    InvalidState,
    Error(Error),
}

impl From<Error> for GoogleOAuth2CallbackErr {
    fn from(err: Error) -> Self {
        GoogleOAuth2CallbackErr::Error(err)
    }
}

impl Message for GoogleOAuth2Callback {
    type Result = Result<UserSession, GoogleOAuth2CallbackErr>;
}

impl Handler<GoogleOAuth2Callback> for AccessExecutor {
    type Result = ResponseFuture<UserSession, GoogleOAuth2CallbackErr>;

    fn handle(&mut self, msg: GoogleOAuth2Callback, _: &mut Self::Context) -> Self::Result {
        let mem = self.mem.clone();
        let db = self.db.clone();
        let settings = &self.settings;
        Box::new(
            google_oauth_client::exchange_code_for_token(
                &msg.code,
                &settings.public_url,
                &settings.google_client_id,
                &settings.google_client_secret,
            )
            .and_then(move |exchange_result: ExchangeResult| {
                let access: &GoogleAccessToken = exchange_result.access_token();

                google_people_client::who_am_i(&access).join(future::ok(exchange_result))
            })
            .map_err(GoogleOAuth2CallbackErr::Error)
            .and_then(move |(i_am, exchange_result)| {
                match exchange_result {
                    ExchangeResult::AccessAndRefreshTokens { access, refresh } => {
                        // Create new token
                        Either::A(
                            db.send(user_tokens::UpsertUserToken {
                                resource_id: i_am.resource_name.clone(),
                                access_token: access.access_token,
                                refresh_token: refresh,
                                token_expiration: access.expires_at,
                            })
                            .map_err(error::ErrorInternalServerError)
                            .flatten()
                            .map_err(GoogleOAuth2CallbackErr::Error)
                            .map(|(user_token, _user_none)| user_token),
                        )
                    }
                    ExchangeResult::AccessTokenOnly(access) => {
                        // Update existing token or revoke
                        Either::B(
                            db.send(user_tokens::GetTokenForResourceId {
                                resource_id: i_am.resource_name.clone(),
                            })
                            .map_err(error::ErrorInternalServerError)
                            .flatten()
                            .map_err(GoogleOAuth2CallbackErr::Error)
                            .and_then(
                                move |token_opt| match token_opt {
                                    Some(token) => Either::A(future::ok(token)),
                                    None => Either::B(
                                        google_oauth_client::revoke_token(&access)
                                            .map_err(GoogleOAuth2CallbackErr::Error)
                                            .and_then(|_| {
                                                Err(GoogleOAuth2CallbackErr::RevokedPreviousTokens)
                                            }),
                                    ),
                                },
                            ),
                        )
                    }
                }
            })
            .and_then(move |token| {
                // Update user session with user token information
                mem.send(user_sessions::GetSessionByKey(msg.state.clone()))
                    .map_err(error::ErrorInternalServerError)
                    .flatten()
                    .map_err(GoogleOAuth2CallbackErr::Error)
                    .and_then(|session_opt: Option<UserSession>| match session_opt {
                        Some(sess) => Ok(sess),
                        None => Err(GoogleOAuth2CallbackErr::InvalidState),
                    })
                    .and_then(move |session: UserSession| {
                        use user_sessions::AddTokenToSessionResult;
                        mem.send(user_sessions::AddTokenToSession {
                            session_key: session.key.clone(),
                            resource_id: token.resource_id.clone(),
                        })
                        .map_err(error::ErrorInternalServerError)
                        .flatten()
                        .map_err(GoogleOAuth2CallbackErr::Error)
                        .and_then(move |res: AddTokenToSessionResult| {
                            match res {
                                AddTokenToSessionResult::SessionNotFound => {
                                    Either::A(future::err(error::ErrorInternalServerError("Session not found when adding token to session")))
                                },
                                AddTokenToSessionResult::Success(user_session) => {
                                    // Added token to session
                                    Either::B(match token.user_id {
                                        None => Either::A(future::ok(user_session)),
                                        Some(token_user_id) => {
                                            use user_sessions::AddUserToSessionResult;
                                            Either::B(mem.send(user_sessions::AddUserToSession {
                                                session_key: session.key,
                                                user_id: token_user_id,
                                            })
                                            .map_err(error::ErrorInternalServerError)
                                            .flatten()
                                            .and_then(move |res: AddUserToSessionResult| {
                                                match res {
                                                    AddUserToSessionResult::SessionNotFound => {
                                                        Err(error::ErrorInternalServerError("Session not found when adding user to session"))
                                                    },
                                                    AddUserToSessionResult::Success(user_session) => {
                                                        // Added token and user to session
                                                        Ok(user_session)
                                                    },
                                                }
                                            }))
                                        }
                                    })
                                }
                            }.map_err(GoogleOAuth2CallbackErr::Error)
                        })
                    })
            }),
        )
    }
}
