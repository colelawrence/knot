use super::MemModel;
use crate::auth::{LoginAccessToken, UserAccessToken};

#[derive(Serialize, Deserialize)]
pub struct MemUser {
    #[serde(rename = "i")]
    user_id: String,
    #[serde(rename = "d")]
    display_name: String,
    #[serde(rename = "f")]
    full_name: Option<String>,
    #[serde(rename = "p")]
    photo_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct UserSession {
    /// User's state key for associating login with session
    #[serde(rename = "k")]
    pub key: UserAccessToken,
    #[serde(rename = "u")]
    pub user: MemUser,
}

impl MemModel for UserSession {
    fn table_prefix() -> &'static str {
        "us"
    }
    fn table_key(&self) -> &str {
        &self.key.0
    }
}

/// Information that could have been filled in by the exchange
#[derive(Serialize, Deserialize, Clone)]
pub struct IAm {
    pub provider: String,
    pub resource_name: String,
    pub email: Option<String>,
    pub given_name: Option<String>,
    pub full_name: Option<String>,
    pub photo_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LoginSession {
    #[serde(rename = "k")]
    pub key: LoginAccessToken,
    #[serde(rename = "h", skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(rename = "a", skip_serializing_if = "Option::is_none")]
    pub i_am: Option<IAm>,
}

impl LoginSession {
    pub fn from_key(key: String) -> Self {
        LoginSession {
            key: LoginAccessToken(key),
            state: None,
            i_am: None,
        }
    }
}

impl MemModel for LoginSession {
    fn table_prefix() -> &'static str {
        "ls"
    }
    fn table_key(&self) -> &str {
        &self.key.0
    }
}

#[derive(Serialize, Deserialize)]
pub struct StateHandoff {
    /// User's handoff key for associating signup with session
    #[serde(rename = "k")]
    pub key: String,
    #[serde(rename = "sk")]
    pub session_key: LoginAccessToken,
}

impl StateHandoff {
    pub fn signup(key: &str, signup_session: &LoginAccessToken) -> Self {
        StateHandoff {
            key: key.to_string(),
            session_key: signup_session.clone(),
        }
    }
}

impl MemModel for StateHandoff {
    fn table_prefix() -> &'static str {
        "sh"
    }
    fn table_key(&self) -> &str {
        &self.key
    }
}

impl Into<crate::auth::User> for MemUser {
    fn into(self) -> crate::auth::User {
        crate::auth::User {
            user_id: self.user_id,
            display_name: self.display_name,
            full_name: self.full_name,
            photo_url: self.photo_url,
        }
    }
}
