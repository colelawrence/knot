use chrono::{DateTime, Utc};

use actix_web::FutureResponse;

pub trait NewToken {
    fn access_token(&self) -> &str;
    fn refresh_token(&self) -> &str;
    fn token_expiration(&self) -> &i64;
    fn resource_name(&self) -> &str;
}

pub trait AccessDb {
    type User;
    type NewUser;
    type UserSession;
    type NewUserToken;
    type UserToken;
    fn create_session(&self) -> FutureResponse<Self::UserSession>;
    fn get_session_by_key(
        &self,
        key: &str,
    ) -> FutureResponse<(Self::UserSession, Option<Self::UserToken>)>;
    fn insert_token_for_session<NT: NewToken>(
        &self,
        session: Self::UserSession,
        new_token: NT,
    ) -> FutureResponse<(Self::UserSession, Self::UserToken, Option<Self::User>)>;
    fn add_token_to_session(
        &self,
        session: Self::UserSession,
        token: &Self::UserToken,
    ) -> FutureResponse<Self::UserSession>;
    fn update_token(
        &self,
        token: Self::UserToken,
        expiration: DateTime<Utc>,
        access_token: &str,
    ) -> FutureResponse<Self::UserToken>;
    fn get_token_for_resource(&self, resource_id: &str) -> FutureResponse<Option<Self::UserToken>>;
    fn insert_user_for_session_and_tokens(
        &self,
        session: Self::UserSession,
        token: Self::UserToken,
        new_user: Self::NewUser,
    ) -> FutureResponse<(Self::UserSession, Self::UserToken, Self::User)>;
    fn update_user(&self, user: Self::User) -> FutureResponse<Self::User>;
}
