//! Redis executor actor
use ::actix::prelude::*;
use actix_web::*;

use futures::future::{self, Either, Future, IntoFuture};

use std::fmt::{Debug, Display};
use actix_web::{error, Error, Result};

// Main application state
use crate::server::State;
use crate::access::{AccessExecutor, CreateGoogleLoginUrl, GoogleLoginUrl, UserSession};

use super::user_auth::{self, RequestUserId, RequestSessionKey};

const USER_SESSION_KEY: &'static str = "user_session";

fn get_google_login_url(req: &HttpRequest<State>) -> FutureResponse<HttpResponse, Error> {
    let State { access, config, ..} = req.state();
    let session_key = req.session_key()?;

    Box::new(
        access.send(CreateGoogleLoginUrl {
            session_key:
        })
    )
}

fn send_error<T: Debug + Display>(e: T) -> Error {
    error::ErrorInternalServerError(format!("Send error: {}; {:?}", e, e))
}

/// Manually revoke application tokens https://myaccount.google.com/permissions
fn login_google_callback(
    request: &HttpRequest<State>,
) -> FutureResponse<HttpResponse, Error> {
    if let Some(cause) = request.query().get("error") {
        return Box::new(future::ok(
            HttpResponse::BadRequest()
                .body(format!("Error during owner authorization: {:?}", cause)),
        ));
    }

    let code = match request.query().get("code") {
        None => return Box::new(future::ok(HttpResponse::BadRequest().body("Missing code"))),
        Some(code) => code.clone(),
    };

    let req_session = request.session();

    let state = request.state();
    let access: Addr<AccessExecutor> = state.access;
    access.send(crate::access::)

}

/// Check which user session you are
fn who_am_i(
    request: &HttpRequest<State>,
) -> Result<HttpResponse> {
    let user_id = request.user_id()?;

    Ok(HttpResponse::Ok()
        .body(user_id))
}

pub fn logout_endpoint(req: &HttpRequest<State>) -> FutureResponse<HttpResponse> {
    let req_session = req.session();
    let mut req = req.clone();
    Box::new(
        future::result(req_session.get::<String>(USER_SESSION_KEY))
        .and_then(|session_key_opt|
        if let Some(session_key) = session_key_opt {
        future::Either::A(req.state().access.send(crate::access::mem::DeleteSessionByKey(session_key))
            .map_err(error::ErrorInternalServerError)
            .flatten()
            .and_then(move |()| {
                req_session.remove(USER_SESSION_KEY);
                Ok(HttpResponse::Found().header("Location", "/").finish())
            }))
    } else {
        future::Either::B(future::ok(HttpResponse::Found().header("Location", "/").finish()))
    }))
}

pub fn login_scope(scope: actix_web::Scope<State>, access: Addr<AccessExecutor>) -> actix_web::Scope<State> {

    scope
        .nested("/v0", |scope|
            scope
                .middleware(user_auth::SessionAuth {
                    access: access,
                })
                .resource("/get_google_login_url", |r| r.f(get_google_login_url))
                .resource("/logout", |r| r.f(logout_endpoint))
                .nested("/me", |scope|
                    scope
                        .middleware(user_auth::UserAuth)
                        .resource("/user_id", |r| r.f(who_am_i))
                )
        )
        .resource("/session/new", |r| r.f(new_session_endpoint))
        .resource("/google/callback", |r| r.f(login_google_callback))
}