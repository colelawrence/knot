use actix::prelude::*;
use actix_web::http::header::{HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use actix_web::http::{HeaderMap, HttpTryFrom};
use actix_web::{error, server, App, FromRequest, HttpRequest, Result};

use actix_web::middleware::{Middleware, Started};
use actix_web::{HttpMessage, HttpResponse};

use futures::{future, Future};
use std::sync::Arc;

use crate::access;

/// `Middleware` for managing a request's user session
pub struct SessionAuth {
    pub access: Addr<access::AccessExecutor>,
}

impl<S: 'static> Middleware<S> for SessionAuth {
    fn start(&self, req: &HttpRequest<S>) -> Result<Started> {
        let authorization_value = req
            .headers()
            .get(AUTHORIZATION)
            .ok_or(error::ErrorBadRequest("Error during owner authorization"))?;

        let token =
            parse_bearer_token(&authorization_value).map_err(|e| error::ErrorBadRequest(e))?;

        let mut req = req.clone();
        let fut = self
            .access
            .send(access::mem::GetSessionByKey(token))
            .map_err(error::ErrorInternalServerError)
            .flatten()
            .and_then(move |sess_opt| match sess_opt {
                Some(sess) => {
                    req.extensions_mut().insert(Arc::new(sess));
                    future::ok(None)
                }
                None => future::err(error::ErrorUnauthorized(
                    "Token was not associated with a user",
                )),
            });
        Ok(Started::Future(Box::new(fut)))
    }
}

/// `Middleware` for ensuring user session has user_id associated
/// `SessionAuth` must be included before this middleware when used.
pub struct UserAuth;

struct UserId(String);

impl<S: 'static> Middleware<S> for UserAuth {
    fn start(&self, req: &HttpRequest<S>) -> Result<Started> {
        let mut req = req.clone();
        if let Some(sess) = req.extensions().get::<Arc<access::UserSession>>() {
            if let Some(user_id) = &sess.user_id {
                req.extensions_mut().insert(UserId(user_id.to_string()));
                return Ok(Started::Done);
            }
        }

        Err(error::ErrorUnauthorized("Must be signed in as a user"))
    }
}

pub trait RequestSessionKey {
    /// Get the session from the request
    fn session_key(&self) -> Result<String>;
}

pub trait RequestUserId {
    /// Get the session from the request
    fn user_id(&self) -> Result<String>;
}

impl<S> RequestUserId for HttpRequest<S> {
    fn user_id(&self) -> Result<String> {
        if let Some(user_id) = self.extensions().get::<UserId>() {
            return Ok(user_id.0.to_string());
        }
        Err(error::ErrorUnauthorized("No user found for tokens"))
    }
}

impl<S> RequestSessionKey for HttpRequest<S> {
    fn session_key(&self) -> Result<String> {
        if let Some(sess) = req.extensions().get::<Arc<access::UserSession>>() {
            if let Some(user_id) = &sess.user_id {
                req.extensions_mut().insert(UserId(user_id.to_string()));
                return Ok(Started::Done);
            }
        }
        Err(error::ErrorUnauthorized("No session key found for tokens"))
    }
}

fn parse_bearer_token(header: &HeaderValue) -> Result<String, &'static str> {
    // "Bearer *" length
    if header.len() < 8 {
        return Err("Bearer token too short");
    }

    let mut parts = header
        .to_str()
        .map_err(|_| "Failed to convert header value to string")?
        .splitn(2, ' ');
    match parts.next() {
        Some("Bearer") => (),
        _ => return Err("Missing \"Bearer \" prefix"),
    }

    let token = parts.next().ok_or("Missing bearer token")?;

    Ok(token.to_string())
}
