use chrono::{DateTime, Utc};

use super::schema::user_logins;

#[derive(Insertable)]
#[table_name = "user_logins"]
pub struct NewUserLogin<'a> {
    pub login_key: &'a str,
    pub user_id: &'a str,
}

#[derive(Queryable, AsChangeset, PartialEq, Debug, Deserialize, Serialize, Clone)]
pub struct UserLogin {
    pub login_key: String,
    pub user_id: String,
}

#[derive(Queryable, AsChangeset, Serialize, Deserialize)]
#[changeset_options(treat_none_as_null="true")]
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
