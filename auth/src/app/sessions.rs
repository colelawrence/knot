use actix_web::{HttpRequest, HttpResponse, Query};
use futures::{future, Future};
use std::sync::Arc;

use super::{AppState, Config};
use crate::auth;
use crate::mem::{models, sessions, MemExecutor};
use crate::prelude::*;

use super::google::clients::{google_oauth_client, google_people_client};

// 10 minutes
const HANDOFF_EXPIRATION: std::time::Duration = std::time::Duration::from_secs(60 * 10);

// Route handlers ↓
pub fn create_login_session(
    req: HttpRequest<AppState>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let mem: MemExecutor = req.state().mem.clone();
    sessions::create_login_access_token(&mem).map(|login_access_token| {
        HttpResponse::Ok().json(json!({
            "access_token": login_access_token,
        }))
    })
}

pub fn login_session_i_am(
    req: HttpRequest<AppState>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    auth::authenticate_login(&req).map(move |login: auth::AuthLogin| {
        HttpResponse::Ok().json(json!({
            "i_am": login.i_am,
        }))
    })
}

pub fn register_login_session(
    req: HttpRequest<AppState>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    auth::authenticate_login(&req).map(move |login: auth::AuthLogin| {
        HttpResponse::Ok().json(json!({
            "i_am": login.i_am,
        }))
    })
}

pub fn user_session_i_am(
    req: HttpRequest<AppState>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    auth::authenticate_user(&req).map(move |user: auth::AuthUser| {
        HttpResponse::Ok().json(json!({
            "user_id": &user.user.user_id,
            "user": user.user,
        }))
    })
}

pub fn create_google_login_url(
    req: HttpRequest<AppState>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let settings: Arc<Config> = req.state().config.clone();
    let mem: MemExecutor = req.state().mem.clone();

    auth::authenticate_login(&req).and_then(move |login: auth::AuthLogin| {
        sessions::create_login_handoff(&mem, &login.access_token)
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
                    sessions::iam_callback(
                        &mem,
                        state.to_string(),
                        models::IAm {
                            email: Some(i_am.email_address),
                            full_name: Some(i_am.display_name),
                            given_name: Some(i_am.given_name),
                            photo_url: Some(i_am.photo_url),
                            resource_name: i_am.resource_name,
                            provider: "goog".to_string(),
                        },
                    )
                })
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
