use actix_web::HttpRequest;
use futures::future::{result, Future};
use std::convert::From;

use super::app::AppState;
use crate::mem::{models, sessions, MemExecutor};
use crate::prelude::*;

use actix_web::http::header::AUTHORIZATION;
const LOGIN_TOKEN_PREFIX: &str = "Login ";
const USER_TOKEN_PREFIX: &str = "User ";

/// TODO: Encode and decode user token values so the same "Token " type can be used for sending and receiving
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(transparent)]
pub struct UserAccessToken(pub String);
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(transparent)]
pub struct LoginAccessToken(pub String);

enum TokenKind {
    Login,
    User,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct User {
    pub user_id: String,
    pub display_name: String,
    pub full_name: Option<String>,
    pub photo_url: Option<String>,
}

pub struct AuthLogin {
    pub access_token: LoginAccessToken,
    pub i_am: Option<models::IAm>,
}

pub struct AuthUser {
    pub access_token: UserAccessToken,
    pub user: User,
}

pub fn authenticate_login(
    req: &HttpRequest<AppState>,
) -> impl Future<Item = AuthLogin, Error = Error> {
    let mem: MemExecutor = req.state().mem.clone();

    result(preprocess_authz_token(req))
        .and_then(move |(kind, token)| match kind {
            TokenKind::Login => Ok(token),
            TokenKind::User => Err(Error::Unauthorized(
                "Requires Login token, but received User token".to_string(),
            )),
        })
        .and_then(move |token| {
            sessions::get_login_session_opt(&mem, &LoginAccessToken(token)).from_err()
        })
        .and_then(|login_session_opt| {
            login_session_opt
                .ok_or(Error::Unauthorized(String::from("Invalid credentials")))
                .map(|login_session: models::LoginSession| AuthLogin {
                    access_token: login_session.key,
                    i_am: login_session.i_am,
                })
        })
}

pub fn authenticate_user(
    req: &HttpRequest<AppState>,
) -> impl Future<Item = AuthUser, Error = Error> {
    let mem: MemExecutor = req.state().mem.clone();

    result(preprocess_authz_token(req))
        .and_then(move |(kind, token)| match kind {
            TokenKind::User => Ok(token),
            TokenKind::Login => Err(Error::Unauthorized(
                "Requires User token, but received Login token".to_string(),
            )),
        })
        .and_then(move |token| {
            sessions::get_user_session_opt(&mem, &UserAccessToken(token)).from_err()
        })
        .and_then(|user_session_opt| {
            user_session_opt
                .ok_or(Error::Unauthorized(String::from("Invalid credentials")))
                .map(|user_session: models::UserSession| AuthUser {
                    access_token: user_session.key,
                    user: user_session.user.into(),
                })
        })
}

fn preprocess_authz_token(req: &HttpRequest<AppState>) -> Result<(TokenKind, String)> {
    let token = match req.headers().get(AUTHORIZATION) {
        Some(token) => token.to_str().unwrap(),
        None => {
            return Err(Error::Unauthorized(
                "No authorization was provided".to_string(),
            ));
        }
    };

    if token.starts_with(LOGIN_TOKEN_PREFIX) {
        let token = token.replacen(LOGIN_TOKEN_PREFIX, "", 1);
        Ok((TokenKind::Login, token))
    } else if token.starts_with(USER_TOKEN_PREFIX) {
        let token = token.replacen(USER_TOKEN_PREFIX, "", 1);
        Ok((TokenKind::User, token))
    } else {
        Err(Error::Unauthorized(
            "Invalid authorization method".to_string(),
        ))
    }
}
