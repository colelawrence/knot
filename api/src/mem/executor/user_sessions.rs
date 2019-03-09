use actix::prelude::*;
use chrono::{DateTime, Utc};
use actix::prelude::*;
use actix_redis::RedisActor;
use actix_web::{error, Error, FutureResponse, Result};

use futures::future::{self, Future};

use super::{mem_error, models, MemExecutor, get_set};

fn get_session_by_key(
    conn: &Addr<RedisActor>,
    by_key: &str,
) -> FutureResponse<Option<models::UserSession>> {
    get_set::get_json(conn, "s", by_key)
}

fn set_session_by_key(
    conn: &Addr<RedisActor>,
    by_key: &str,
    value: &models::UserSession,
) -> FutureResponse<()> {
    get_set::set_json(conn, "s", by_key, value)
}

pub struct CreateSession();

impl Message for CreateSession {
    type Result = Result<models::UserSession>;
}

impl Handler<CreateSession> for MemExecutor {
    type Result = FutureResponse<models::UserSession>;

    fn handle(&mut self, msg: CreateSession, _: &mut Self::Context) -> Self::Result {
        let conn = self.conn();

        // Get previous user token if it exists
        let user_tokens_opt: Option<models::UserToken> =
            get_token_by_resource_id(&conn, &msg.resource_id)?;

        // Retrieve original user
        let existing_user_opt: Option<models::User> = if let Some(existing_tokens) = user_tokens_opt
        {
            // User exists
            use schema::user_tokens::dsl::*;
            // Delete previous user token
            diesel::delete(user_tokens.filter(resource_id.eq(&existing_tokens.resource_id)))
                .execute(&conn)
                .map_err(|e| db_error("UpsertUserToken: Error deleting previous user_tokens", e))?;

            // Retrieve previous user if available
            if let Some(existing_user_id) = existing_tokens.user_id {
                use schema::users::dsl::*;
                users
                    .filter(id.eq(&existing_user_id))
                    .get_result(&conn)
                    .optional()
                    .map_err(|e| {
                        db_error(
                            "UpsertUserToken: Error retrieving previous existing user",
                            e,
                        )
                    })?
            } else {
                None
            }
        } else {
            None
        };

        let existing_user_id_opt = existing_user_opt.as_ref().map(|u| u.id.clone());

        let new_user_token = models::NewUserToken {
            resource_id: &msg.resource_id,
            access_token: &msg.access_token,
            refresh_token: &msg.refresh_token,
            token_expiration: &msg.token_expiration,
            // ensure if upserted that the new token are associated with the original user
            user_id: existing_user_id_opt.as_ref(),
        };

        let inserted_token: models::UserToken = diesel::insert_into(schema::user_tokens::table)
            .values(&new_user_token)
            .get_result(&conn)
            .map_err(|e| db_error("UpsertUserToken: Error inserting user token", e))?;

        Ok((inserted_token, existing_user_opt))
    }
}

pub struct GetTokenForResourceId {
    resource_id: String,
}

impl Message for GetTokenForResourceId {
    type Result = Result<Option<models::UserToken>>;
}

impl Handler<GetTokenForResourceId> for MemExecutor {
    type Result = Result<Option<models::UserToken>>;

    fn handle(&mut self, msg: GetTokenForResourceId, _: &mut Self::Context) -> Self::Result {
        let conn = self.conn();
        get_token_by_resource_id(&conn, &msg.resource_id)
    }
}

pub struct UpsertUserWithToken {
    pub display_name: String,
    pub full_name: Option<String>,
    pub photo_url: Option<String>,
    pub is_person: bool,
    pub token: models::UserToken,
}

impl Message for UpsertUserWithToken {
    type Result = Result<(models::UserToken, models::User)>;
}

impl Handler<UpsertUserWithToken> for MemExecutor {
    type Result = Result<(models::UserToken, models::User)>;

    fn handle(&mut self, msg: UpsertUserWithToken, _: &mut Self::Context) -> Self::Result {
        let conn = self.conn();
        // 1. Get Token
        let mut token: models::UserToken = {
            use schema::user_tokens::dsl::*;
            user_tokens
                .filter(resource_id.eq(msg.token.resource_id))
                .get_result(&conn)
                .map_err(|e| db_error("UpsertUserWithToken: Error retrieving user token", e))?
        };
        // 2. Check existing user
        let user_exists_opt: Option<models::User> = if let Some(ref user_id) = token.user_id {
            get_user_by_id(&conn, &user_id)?
        } else {
            None
        };

        let user: models::User = if let Some(mut user_exists) = user_exists_opt {
            // 2.Exists: Update user with info
            use schema::users;
            user_exists.full_name = msg.full_name.or(user_exists.full_name);
            user_exists.photo_url = msg.photo_url.or(user_exists.photo_url);
            diesel::update(users::table)
                .set(&user_exists)
                .get_result(&conn)
                .map_err(|e| db_error("UpsertUserWithToken: Error updating existing user", e))?
        } else {
            // 2.DNE.1: Create user with info
            use schema::users;
            let new_user = models::NewUser {
                display_name: &msg.display_name,
                full_name: msg.full_name.as_ref(),
                photo_url: msg.photo_url.as_ref(),
                is_person: msg.is_person,
            };

            diesel::insert_into(users::table)
                .values(new_user)
                .get_result(&conn)
                .map_err(|e| db_error("UpsertUserWithToken: Error inserting new user", e))?
        };

        // 3. Update token to point at user_id
        let updated_token: models::UserToken = {
            token.user_id = Some(user.id.clone());

            use schema::user_tokens;
            diesel::update(user_tokens::table)
                .set(&token)
                .get_result(&conn)
                .map_err(|e| db_error("UpsertUserWithToken: Error updating user token", e))?
        };

        Ok((updated_token, user))
    }
}

pub struct UpdateUser {
    pub user_id: String,
    pub display_name: String,
    pub full_name: Option<String>,
    pub photo_url: Option<String>,
}

impl Message for UpdateUser {
    type Result = Result<models::User>;
}

impl Handler<UpdateUser> for MemExecutor {
    type Result = Result<models::User>;

    fn handle(&mut self, msg: UpdateUser, _: &mut Self::Context) -> Self::Result {
        let conn = self.conn();

        // 2.Exists: Update user with info
        use schema::users::dsl::*;
        diesel::update(schema::users::table)
            .filter(id.eq(msg.user_id))
            .set((
                display_name.eq(msg.display_name),
                full_name.eq(msg.full_name),
                photo_url.eq(msg.photo_url),
            ))
            .get_result(&conn)
            .map_err(|e| db_error("UpdateUser: Error updating existing user", e))
    }
}

pub struct GetUserById {
    user_id: String,
}

impl Message for GetUserById {
    type Result = Result<Option<models::User>>;
}

impl Handler<GetUserById> for MemExecutor {
    type Result = Result<Option<models::User>>;

    fn handle(&mut self, msg: GetUserById, _: &mut Self::Context) -> Self::Result {
        let conn = self.conn();
        get_user_by_id(&conn, &msg.user_id)
    }
}
