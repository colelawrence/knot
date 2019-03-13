use super::MemModel;

#[derive(Clone, Serialize, Deserialize)]
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
    pub key: String,
    #[serde(rename = "u")]
    pub user: MemUser,
}

impl UserSession {
    pub fn from_key_and_user(key: String, mem_user: MemUser) -> Self {
        UserSession {
            key: key,
            user: mem_user,
        }
    }
}

impl MemModel for UserSession {
    fn table_prefix() -> &'static str {
        "us"
    }
    fn table_key(&self) -> &str {
        &self.key
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
    pub key: String,
    #[serde(rename = "a", skip_serializing_if = "Option::is_none")]
    pub i_am: Option<IAm>,
    #[serde(rename = "u", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

impl LoginSession {
    pub fn from_key(key: String) -> Self {
        LoginSession {
            key: key,
            i_am: None,
            user_id: None,
        }
    }
}

impl MemModel for LoginSession {
    fn table_prefix() -> &'static str {
        "ls"
    }
    fn table_key(&self) -> &str {
        &self.key
    }
}

#[derive(Serialize, Deserialize)]
pub struct StateHandoff {
    /// User's handoff key for associating signup with session
    #[serde(rename = "k")]
    pub key: String,
    #[serde(rename = "sk")]
    pub session_key: String,
}

impl StateHandoff {
    pub fn login(key: &str, login_access_key: &str) -> Self {
        StateHandoff {
            key: key.to_string(),
            session_key: login_access_key.to_string(),
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

use crate::db;

impl From<db::models::User> for MemUser {
    fn from(user: db::models::User) -> Self {
        MemUser {
            user_id: user.id,
            display_name: user.display_name,
            full_name: user.full_name,
            photo_url: user.photo_url,
        }
    }
}
