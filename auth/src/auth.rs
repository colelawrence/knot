use actix_web::{FromRequest, HttpRequest};
use futures::future::{result, Future};
use std::convert::From;
use std::iter::Iterator;

use super::app::AppState;
use crate::mem::{models, sessions, MemExecutor};
use crate::prelude::*;

use actix_web::http::header::AUTHORIZATION;
const BEARER_TOKEN_PREFIX: &str = "Bearer ";

#[derive(Debug, Clone)]
pub struct UserAccessKey(pub String);
#[derive(Debug, Clone)]
pub struct LoginAccessKey(pub String);

pub struct AccessKey {
    salt: String,
    key: AccessKeyInner,
}

enum AccessKeyInner {
    Login(LoginAccessKey),
    User(UserAccessKey),
}

/// The Access token is an opaque value which can be decrypted to find the user's session key
/// The key is then used to query against the memory database.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(transparent)]
pub struct AccessToken(String);

use crate::utils::{dec, enc, hex, secure_rand};

const SALT_LENGTH: usize = 16;
const SALT_LENGTH_HEX: usize = 32;

const ACCESS_KEY_LOGIN_PREFIX: &str = "Login ";
const ACCESS_KEY_USER_PREFIX: &str = "User ";
impl AccessToken {
    pub fn encrypt(access_key: AccessKey, pepper: &str) -> Self {
        let AccessKey { salt, key } = access_key;
        let fm = match key {
            AccessKeyInner::Login(login_key) => {
                format!("{}{}", ACCESS_KEY_LOGIN_PREFIX, login_key.0)
            }
            AccessKeyInner::User(user_key) => format!("{}{}", ACCESS_KEY_USER_PREFIX, user_key.0),
        };
        let salt_and_pepper = format!("{}{}", salt, pepper);
        let sp_encrypted = enc(fm.as_bytes(), &salt_and_pepper);
        let salt_and_sp_encrypted = format!("{}{}", salt, sp_encrypted);
        AccessToken(enc(salt_and_sp_encrypted.as_bytes(), "access_token"))
    }

    pub fn decrypt(&self, pepper: &str) -> Result<AccessKey, &'static str> {
        let salt_and_sp_encrypted = dec(&self.0, "access_token")?;
        if salt_and_sp_encrypted.len() <= SALT_LENGTH_HEX {
            return Err("Token too short");
        }
        use std::str::from_utf8;
        let (salt_utf8, sp_encrypted_utf8) = salt_and_sp_encrypted.split_at(SALT_LENGTH_HEX);
        let (salt, sp_encrypted) = (
            from_utf8(salt_utf8).map_err(|_| "Invalid utf8")?,
            from_utf8(&sp_encrypted_utf8).map_err(|_| "Invalid utf8")?,
        );
        let salt_and_pepper = format!("{}{}", salt, pepper);
        let fm_utf8 = dec(sp_encrypted, &salt_and_pepper)?;
        let fm = from_utf8(&fm_utf8).map_err(|_| "Invalid utf8")?;
        let key = if fm.starts_with(ACCESS_KEY_LOGIN_PREFIX) {
            let (_, key) = fm.split_at(ACCESS_KEY_LOGIN_PREFIX.len());
            AccessKeyInner::Login(LoginAccessKey(key.to_string()))
        } else if fm.starts_with(ACCESS_KEY_USER_PREFIX) {
            let (_, key) = fm.split_at(ACCESS_KEY_USER_PREFIX.len());
            AccessKeyInner::User(UserAccessKey(key.to_string()))
        } else {
            return Err("Unknown login key kind");
        };

        Ok(AccessKey {
            salt: salt.to_string(),
            key,
        })
    }
}

impl AccessKey {
    pub fn new_user_key(key: &str) -> Self {
        let new_salt = hex(&secure_rand(SALT_LENGTH));
        AccessKey {
            salt: new_salt,
            key: AccessKeyInner::User(UserAccessKey(key.to_string())),
        }
    }

    pub fn new_login_key(key: LoginAccessKey) -> Self {
        let new_salt = hex(&secure_rand(SALT_LENGTH));
        AccessKey {
            salt: new_salt,
            key: AccessKeyInner::Login(key),
        }
    }

    fn login_key(&self) -> Option<&LoginAccessKey> {
        match self.key {
            AccessKeyInner::Login(ref l) => Some(&l),
            _ => None,
        }
    }

    fn user_key(&self) -> Option<&UserAccessKey> {
        match self.key {
            AccessKeyInner::User(ref l) => Some(&l),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct User {
    pub user_id: String,
    pub display_name: String,
    pub full_name: Option<String>,
    pub photo_url: Option<String>,
}

pub struct AuthLogin {
    pub access_key: LoginAccessKey,
    pub i_am: Option<models::IAm>,
    pub user_id: Option<String>,
}

pub struct AuthUser {
    pub access_key: UserAccessKey,
    pub user: User,
}

impl FromRequest<AppState> for AuthLogin {
    type Config = ();
    type Result = Box<Future<Item = AuthLogin, Error = actix_web::Error>>;

    #[inline]
    fn from_request(req: &HttpRequest<AppState>, _cfg: &Self::Config) -> Self::Result {
        Box::new(
            authenticate_login(&req)
                .map_err(|_| actix_web::error::ErrorUnauthorized("Invalid login session token")),
        )
    }
}

impl FromRequest<AppState> for AuthUser {
    type Config = ();
    type Result = Box<Future<Item = AuthUser, Error = actix_web::Error>>;

    #[inline]
    fn from_request(req: &HttpRequest<AppState>, _cfg: &Self::Config) -> Self::Result {
        Box::new(
            authenticate_user(&req)
                .map_err(|_| actix_web::error::ErrorUnauthorized("Invalid user session token")),
        )
    }
}

fn authenticate_login(
    req: &HttpRequest<AppState>,
) -> impl Future<Item = AuthLogin, Error = Error> {
    let mem: MemExecutor = req.state().mem.clone();
    let pepper = req.state().config.pepper_0.clone();

    result(preprocess_authz_token(req))
        .and_then(move |access_token| {
            access_token.decrypt(&pepper).map_err(|err| {
                debug!("authenticate_login: Decrypt error \"{}\"", err);
                Error::BadRequest(format!("Authentication value error"))
            })
        })
        .and_then(move |access_key: AccessKey| {
            access_key
                .login_key()
                .map(std::clone::Clone::clone)
                .ok_or(Error::BadRequest("Not a login access token".to_string()))
        })
        .and_then(move |login_access_key: LoginAccessKey| {
            sessions::get_login_session_opt(&mem, &login_access_key).from_err()
        })
        .and_then(|login_session_opt| {
            login_session_opt
                .ok_or(Error::Unauthorized(String::from("Invalid credentials")))
                .map(|login_session: models::LoginSession| AuthLogin {
                    access_key: LoginAccessKey(login_session.key),
                    i_am: login_session.i_am,
                    user_id: None,
                })
        })
}

fn authenticate_user(
    req: &HttpRequest<AppState>,
) -> impl Future<Item = AuthUser, Error = Error> {
    let mem: MemExecutor = req.state().mem.clone();
    let pepper = req.state().config.pepper_0.clone();

    result(preprocess_authz_token(req))
        .and_then(move |access_token| {
            access_token.decrypt(&pepper).map_err(|err| {
                debug!("authenticate_login: Decrypt error \"{}\"", err);
                Error::BadRequest(format!("Authentication value error"))
            })
        })
        .and_then(move |access_key: AccessKey| {
            access_key
                .user_key()
                .map(std::clone::Clone::clone)
                .ok_or(Error::BadRequest("Not a user access token".to_string()))
        })
        .and_then(move |user_key: UserAccessKey| {
            sessions::get_user_session_opt(&mem, &user_key).from_err()
        })
        .and_then(|user_session_opt| {
            user_session_opt
                .ok_or(Error::Unauthorized(String::from("Invalid credentials")))
                .map(|user_session: models::UserSession| AuthUser {
                    access_key: UserAccessKey(user_session.key),
                    user: user_session.user.into(),
                })
        })
}

fn preprocess_authz_token(req: &HttpRequest<AppState>) -> Result<AccessToken> {
    let token = match req.headers().get(AUTHORIZATION) {
        Some(token) => token.to_str().unwrap(),
        None => {
            return Err(Error::Unauthorized(
                "No authorization was provided".to_string(),
            ));
        }
    };

    if token.starts_with(BEARER_TOKEN_PREFIX) {
        let token = token.replacen(BEARER_TOKEN_PREFIX, "", 1);
        Ok(AccessToken(token))
    } else {
        Err(Error::Unauthorized(
            "Invalid authorization method".to_string(),
        ))
    }
}
