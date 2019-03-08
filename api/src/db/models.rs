use chrono::{DateTime, Utc};

#[derive(Queryable, Serialize, Deserialize)]
pub struct UserSession {
    pub id: i32,
    /// User's state key for associating login with session
    pub key: String,
    pub created_at: DateTime<Utc>,
    pub user_token_resource_id: Option<String>,
    pub user_id: Option<String>,
}

use super::schema::user_tokens;

#[derive(Insertable)]
#[table_name = "user_tokens"]
pub struct NewUserToken<'a> {
    pub resource_id: &'a str,
    pub access_token: &'a str,
    pub refresh_token: &'a str,
    pub token_expiration: &'a DateTime<Utc>,
    pub user_id: Option<&'a String>,
}

#[derive(Queryable, AsChangeset, PartialEq, Debug, Deserialize, Serialize, Clone)]
pub struct UserToken {
    pub resource_id: String,
    pub access_token: String,
    pub refresh_token: String,
    pub token_expiration: DateTime<Utc>,
    pub user_id: Option<String>,
}

#[derive(Queryable, AsChangeset, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub display_name: String,
    pub full_name: Option<String>,
    pub photo_url: Option<String>,
    pub is_person: bool,
    pub created_at: DateTime<Utc>,
}

use super::schema::users;

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub display_name: &'a str,
    pub full_name: Option<&'a String>,
    pub photo_url: Option<&'a String>,
    pub is_person: bool,
}
