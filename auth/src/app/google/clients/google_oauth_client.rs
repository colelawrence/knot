use chrono::{Duration, Utc};

use actix_web::{client, error, FutureResponse, HttpMessage};
use futures::{future, Future};

use super::GoogleAccessToken;

#[derive(Deserialize, Debug)]
struct GoogleTokenAuthCodeJson {
    // success
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
    pub token_type: Option<String>,
    // error
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug)]
pub enum ExchangeResult {
    AccessTokenOnly(GoogleAccessToken),
    AccessAndRefreshTokens {
        access: GoogleAccessToken,
        refresh: String,
    },
}

impl ExchangeResult {
    pub fn access_token(&self) -> &GoogleAccessToken {
        match self {
            ExchangeResult::AccessAndRefreshTokens { ref access, .. } => access,
            ExchangeResult::AccessTokenOnly(ref access) => access,
        }
    }
}

pub fn exchange_code_for_token(
    code: &str,
    redirect_uri: &str,
    client_id: &str,
    client_secret: &str,
) -> FutureResponse<ExchangeResult> {
    // Construct a request against http://localhost:8020/token, the access token endpoint
    let google_token_endpoint = "https://www.googleapis.com/oauth2/v4/token";

    // https://developers.google.com/identity/protocols/OAuth2WebServer#offline
    let params = [
        ("code", code.as_ref()),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("redirect_uri", &redirect_uri),
        ("grant_type", "authorization_code"),
    ];

    // Not sure why "Accept-Encoding" "identity" works to make it resolve far more quickly
    // https://github.com/actix/actix-web/issues/674#issuecomment-466720953

    Box::new(
        client::post(google_token_endpoint)
            .header("User-Agent", "Actix-web")
            .header("Accept-Encoding", "identity")
            .form(&params)
            .unwrap()
            .send()
            .timeout(std::time::Duration::from_secs(10))
            .map_err(|e| {
                warn!("Failed to send code params for Token exchange: {:?}", e);
                error::ErrorFailedDependency("Code exchange send error")
            })
            .and_then(|resp: actix_web::client::ClientResponse| {
                info!("exchange_code_for_token client response json {:?}", resp);
                if resp.status().is_success() {
                    future::Either::A(resp.json::<GoogleTokenAuthCodeJson>().map_err(|e| {
                        warn!("Failed to parse GoogleTokenAuthCodeJson {:?}", e);
                        error::ErrorFailedDependency("Code exchange json parse error")
                    }))
                } else {
                    future::Either::B(future::err(error::ErrorBadRequest(format!(
                        "Code exchange request error [{}], please try again",
                        resp.status()
                    ))))
                }
            })
            .and_then(move |token_map: GoogleTokenAuthCodeJson| {
                info!("exchange_code_for_token token_map matching");
                match (token_map.access_token, token_map.expires_in) {
                    (Some(access), Some(expires_in)) => {
                        let expires_at = Utc::now() + Duration::seconds(expires_in);
                        let access_token = GoogleAccessToken {
                            access_token: access,
                            expires_at,
                        };
                        Ok(match token_map.refresh_token {
                            Some(refresh) => ExchangeResult::AccessAndRefreshTokens {
                                access: access_token,
                                refresh,
                            },
                            None => ExchangeResult::AccessTokenOnly(access_token),
                        })
                    }
                    _ => Err(error::ErrorInternalServerError(format!(
                        "Error with received tokens: {}",
                        token_map
                            .error
                            .or(token_map.error_description)
                            .unwrap_or("Access token missing".to_string())
                    ))),
                }
            }),
    )
}

pub fn get_login_url(
    state: &str,
    redirect_uri: &str,
    client_id: &str,
    domain: Option<&str>,
) -> String {
    let oauth_endpoint = "https://accounts.google.com/o/oauth2/v2/auth";
    // let calendar_scope = "https://www.googleapis.com/auth/calendar";
    // let emails_readonly_scope = "https://www.googleapis.com/auth/user.emails.read";
    let profile_scope = "https://www.googleapis.com/auth/userinfo.profile";
    let scopes = format!("{}", profile_scope);
    let nonce = crate::utils::secure_rand_hex(8);

    format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&hd={}&nonce={}&prompt=select_account",
        oauth_endpoint, client_id, redirect_uri, scopes, state, domain.unwrap_or(""), nonce
    )
}

// Refreshing a token
// https://developers.google.com/identity/protocols/OAuth2WebServer#offline
#[derive(Deserialize, Debug)]
struct GoogleTokenRefresh {
    pub access_token: String, // "1/fFAGRNJru1FTz70BzhT3Zg",
    pub expires_in: i64,      //  3920,
    pub token_type: String,   // "Bearer"
}

pub fn refresh_google_token(
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> FutureResponse<GoogleAccessToken> {
    let google_token_endpoint = "https://www.googleapis.com/oauth2/v4/token";
    // let google_token_endpoint = "http://httpbin.org/post";

    // https://developers.google.com/identity/protocols/OAuth2WebServer#offline
    let params = [
        ("refresh_token", refresh_token),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("grant_type", "refresh_token"),
    ];

    Box::new(
        client::post(google_token_endpoint)
            .header("User-Agent", "Actix-web")
            .header("Accept-Encoding", "identity")
            .form(&params)
            .unwrap()
            .send()
            .timeout(std::time::Duration::from_secs(10))
            .map_err(|e| {
                warn!("Failed to send refresh token for Token refresh: {:?}", e);
                error::ErrorInternalServerError("Token refresh send error")
            })
            .and_then(|resp: actix_web::client::ClientResponse| {
                if resp.status().is_success() {
                    future::Either::A(resp.json::<GoogleTokenRefresh>().map_err(|e| {
                        warn!("Failed to parse GoogleTokenAuthCodeJson {:?}", e);
                        error::ErrorInternalServerError("Token refresh json parse error")
                    }))
                } else {
                    future::Either::B(future::err(error::ErrorBadRequest(format!(
                        "Token refresh request error [{}], please try again",
                        resp.status()
                    ))))
                }
            })
            .map(move |resp_json: GoogleTokenRefresh| GoogleAccessToken {
                access_token: resp_json.access_token,
                expires_at: Utc::now() + Duration::seconds(resp_json.expires_in),
            }),
    )
}

pub fn revoke_token(token: &GoogleAccessToken) -> FutureResponse<()> {
    let google_token_endpoint = "https://accounts.google.com/o/oauth2/revoke";

    // https://developers.google.com/identity/protocols/OAuth2WebServer#offline
    let url = format!("{}?token={}", google_token_endpoint, &token.access_token);
    Box::new(
        client::get(&url)
            .header("User-Agent", "Actix-web")
            .header("Accept-Encoding", "identity")
            .finish()
            .unwrap()
            .send()
            .timeout(std::time::Duration::from_secs(10))
            .map_err(|e| {
                warn!("Error revoking token: {:?}", e);
                error::ErrorInternalServerError("Error revoking token")
            })
            .map(|_| {
                info!("Successfully revoked user's tokens");
                ()
            }),
    )
}
