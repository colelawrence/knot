use actix::prelude::*;
use actix_web::{HttpRequest, HttpResponse, Query};
use futures::{
    future::{self, Either},
    Future,
};
use std::sync::Arc;

use super::{AppState, Config};
use crate::auth;
use crate::mem::{models, sessions, MemExecutor};
use crate::prelude::*;

use crate::db::{self, users, DbExecutor};

use super::google::clients::{google_oauth_client, google_people_client};

// Route handlers ↓
pub fn create_login_session(
    req: HttpRequest<AppState>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let mem: MemExecutor = req.state().mem.clone();
    let pepper = req.state().config.pepper_0.clone();
    sessions::create_login_access_key(&mem).map(move |login_access_key| {
        let access_key = auth::AccessKey::new_login_key(login_access_key);
        HttpResponse::Ok().json(json!({
            "access_token": auth::AccessToken::encrypt(access_key, &pepper),
        }))
    })
}

pub fn login_session_i_am(login: auth::AuthLogin) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "i_am": login.i_am,
        "user_id": login.user_id,
    }))
}

pub fn register_login_session(
    (login, db, mem): (auth::AuthLogin, Addr<DbExecutor>, MemExecutor),
) -> impl Future<Item = HttpResponse, Error = Error> {
    if login.user_id.is_some() {
        Either::A(future::ok(HttpResponse::Ok().json(json!({
            "success": "You already have a user!",
        }))))
    } else {
        let login_key = login.access_key;
        Either::B(if let Some(i_am) = login.i_am {
            let full_name = i_am.full_name.clone();
            Either::A(if i_am.provider != "google" {
                Either::A(future::err(Error::BadRequest(format!(
                    "{} as a login provider is not fully supported",
                    i_am.provider
                ))))
            } else {
                Either::B(
                    db.send(users::CreateUser {
                        external_id: users::ExtResourceId::google(&i_am.resource_name),
                        display_name: i_am
                            .given_name
                            .or(i_am.full_name)
                            .unwrap_or("New User".to_string()),
                        full_name: full_name,
                        photo_url: i_am.photo_url,
                    })
                    .flatten()
                    .and_then(move |user| {
                        let user_id = user.id.clone();
                        sessions::link_login_session_to_user_id(&mem, &login_key, user_id)
                            .map(move |_| user)
                    })
                    .map(|user| {
                        HttpResponse::Ok().json(json!({
                            "success": "Registered new user",
                            "user": user,
                        }))
                    }),
                )
            })
        } else {
            Either::B(future::err(Error::BadRequest(String::from(
                "Session has not logged in yet",
            ))))
        })
    }
}

pub fn create_user_session(
    (login, req, db): (auth::AuthLogin, HttpRequest<AppState>, Addr<DbExecutor>),
) -> impl Future<Item = HttpResponse, Error = Error> {
    match login.user_id {
        None => Either::A(future::err(Error::Unauthorized(String::from(
            "Login session is not associated with a user",
        )))),
        Some(user_id) => Either::B(
            db.send(users::GetUserById { user_id })
                .flatten()
                .and_then(|db_user_opt| {
                    db_user_opt.ok_or(Error::BadRequest(String::from(
                        "User linked no longer exists",
                    )))
                })
                .and_then(move |db_user| {
                    let mem: MemExecutor = req.state().mem.clone();
                    let pepper = req.state().config.pepper_0.clone();
                    sessions::create_user_access_key(&mem, db_user).map(move |user_access_key| {
                        let access_key = auth::AccessKey::new_user_key(user_access_key);
                        HttpResponse::Ok().json(json!({
                            "access_token": auth::AccessToken::encrypt(access_key, &pepper),
                        }))
                    })
                }),
        ),
    }
}

pub fn user_session_i_am(user: auth::AuthUser) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "user_id": &user.user.user_id,
        "user": user.user,
    }))
}

pub fn create_google_login_url(
    (login, req): (auth::AuthLogin, HttpRequest<AppState>),
) -> impl Future<Item = HttpResponse, Error = Error> {
    let settings: Arc<Config> = req.state().config.clone();
    let mem: MemExecutor = req.state().mem.clone();

    sessions::create_login_handoff(&mem, &login.access_key)
        .map(move |handoff_state: sessions::HandoffState| {
            google_oauth_client::get_login_url(
                &handoff_state.0,
                &google_redirect_uri(&settings.http_public_url),
                &settings.google_oauth_client_id,
                None,
            )
        })
        .map(|login_url| {
            HttpResponse::Ok().json(json!({
                "url": login_url,
            }))
        })
}

#[derive(Debug, Deserialize)]
pub struct GoogleCallbackQuery {
    error: Option<String>,
    code: Option<String>,
    state: Option<String>,
}

pub fn google_callback(
    (query, req): (Query<GoogleCallbackQuery>, HttpRequest<AppState>),
) -> AppFuture<HttpResponse> {
    if let Some(ref cause) = query.error {
        return Box::new(future::err(Error::BadRequest(format!(
            "Error during login: {:?}",
            cause
        ))));
    }
    if let Some(ref code) = query.code {
        if let Some(ref state) = query.state {
            // with code and state, we can make the exchange
            let settings: Arc<Config> = req.state().config.clone();
            let mem = req.state().mem.clone();
            let db: Addr<DbExecutor> = req.state().db.clone();

            let (code, state) = (code.to_string(), state.to_string());

            let redirect_uri = google_redirect_uri(&settings.http_public_url);
            Box::new(
                google_oauth_client::exchange_code_for_token(
                    &code.clone(),
                    &redirect_uri,
                    &settings.google_oauth_client_id,
                    &settings.google_oauth_client_secret,
                )
                .and_then(|access_result: google_oauth_client::ExchangeResult| {
                    google_people_client::who_am_i(access_result.access_token())
                })
                .map_err(|_| Error::InternalServerError)
                .and_then(move |i_am: google_people_client::IAm| {
                    db.send(users::GetLoginForResource(users::ExtResourceId::google(
                        &i_am.resource_name,
                    )))
                    .flatten()
                    .join(future::ok(i_am))
                })
                .and_then(
                    move |(user_login_opt, i_am): (
                        Option<db::models::UserLogin>,
                        google_people_client::IAm,
                    )| {
                        if let Some(user_login) = user_login_opt {
                            Either::A(sessions::link_state_to_user_id(
                                &mem,
                                state.to_string(),
                                user_login.user_id,
                            ))
                        } else {
                            Either::B(sessions::link_state_to_i_am(
                                &mem,
                                state.to_string(),
                                models::IAm {
                                    email: Some(i_am.email_address),
                                    full_name: Some(i_am.display_name),
                                    given_name: Some(i_am.given_name),
                                    photo_url: Some(i_am.photo_url),
                                    resource_name: i_am.resource_name,
                                    provider: "google".to_string(),
                                },
                            ))
                        }
                    },
                )
                .map(|_| HttpResponse::Found().header("Location", "/").finish()),
            )
        } else {
            Box::new(future::err(Error::BadRequest(format!("Missing state"))))
        }
    } else {
        Box::new(future::err(Error::BadRequest(format!("Missing code"))))
    }
}

fn google_redirect_uri(public_url: &str) -> String {
    format!("{}/auth/v0/google/callback", public_url)
}
