use super::{models, schema, DbExecutor};
use crate::prelude::*;
use actix::prelude::*;
use diesel::prelude::*;

fn get_user_login_by_ext_id(
    conn: &PgConnection,
    by_ext_id: &ExtResourceId,
) -> Result<Option<models::UserLogin>> {
    use schema::user_logins::dsl::*;

    user_logins
        .filter(external_id.eq(by_ext_id.to_string()))
        .get_result(conn)
        .optional()
        .map_err(|e| db_error("get_user_login_by_ext_id: get_result error", e))
}

fn get_user_by_id(conn: &PgConnection, by_id: &str) -> Result<Option<models::User>> {
    use schema::users::dsl::*;

    users
        .filter(id.eq(by_id))
        .get_result(conn)
        .optional()
        .map_err(|e| db_error("get_user_by_id: get_result error", e))
}

pub struct ExtResourceId {
    provider: String,
    resource_name: String,
}

impl ExtResourceId {
    pub fn google(resource_name: &str) -> Self {
        ExtResourceId {
            provider: String::from("goog"),
            resource_name: resource_name.to_string(),
        }
    }

    fn to_string(&self) -> String {
        format!("{}|{}", self.provider, self.resource_name)
    }
}

pub struct GetLoginForResource(pub ExtResourceId);

impl Message for GetLoginForResource {
    type Result = Result<Option<models::UserLogin>>;
}

impl Handler<GetLoginForResource> for DbExecutor {
    type Result = Result<Option<models::UserLogin>>;

    fn handle(&mut self, msg: GetLoginForResource, _: &mut Self::Context) -> Self::Result {
        let conn = self.conn()?;

        get_user_login_by_ext_id(&conn, &msg.0)
    }
}

pub struct CreateUser {
    pub external_id: ExtResourceId,
    pub display_name: String,
    pub full_name: Option<String>,
    pub photo_url: Option<String>,
}

impl Message for CreateUser {
    type Result = Result<models::User>;
}

impl Handler<CreateUser> for DbExecutor {
    type Result = Result<models::User>;

    fn handle(&mut self, msg: CreateUser, _: &mut Self::Context) -> Self::Result {
        let conn = self.conn()?;

        // 1. Ensure User Login does not exist
        if let Some(_) = get_user_login_by_ext_id(&conn, &msg.external_id)? {
            return Err(Error::BadRequest(String::from(
                "User associated to that login method already exists",
            )));
        }

        // 2. Create User
        use schema::users;
        let new_user = models::NewUser {
            display_name: &msg.display_name,
            full_name: msg.full_name.as_ref(),
            photo_url: msg.photo_url.as_ref(),
            is_person: true,
        };

        let created_user: models::User = diesel::insert_into(users::table)
            .values(new_user)
            .get_result(&conn)
            .map_err(|e| db_error("CreateUser: Error inserting new user", e))?;

        // 3. Create User Login
        use schema::user_logins;
        diesel::insert_into(user_logins::table)
            .values(models::NewUserLogin {
                external_id: &msg.external_id.to_string(),
                user_id: &created_user.id,
            })
            .execute(&conn)
            .map_err(|e| db_error("CreateUser: Error updating user token", e))?;

        Ok(created_user)
    }
}

fn db_error<T: Into<String>, U: std::fmt::Debug>(message: T, err: U) -> Error {
    let mstr = message.into();
    error!("db_error: {}; {:?}", mstr, err);
    Error::InternalServerError
}

pub struct UpdateUser {
    pub user_id: String,
    pub display_name: Option<String>,
    pub full_name: Option<Option<String>>,
    pub photo_url: Option<Option<String>>,
}

impl Message for UpdateUser {
    type Result = Result<models::User>;
}

impl Handler<UpdateUser> for DbExecutor {
    type Result = Result<models::User>;

    fn handle(&mut self, msg: UpdateUser, _: &mut Self::Context) -> Self::Result {
        let conn = self.conn()?;

        // 2.Exists: Update user with info
        use schema::users::dsl::*;
        let mut user = users
            .filter(id.eq(&msg.user_id))
            .get_result::<models::User>(&conn)
            .map_err(|e| db_error("UpdateUser: Error retrieving user", e))?;

        if let Some(display_name_value) = msg.display_name {
            user.display_name = display_name_value;
        }
        if let Some(full_name_value) = msg.full_name {
            user.full_name = full_name_value;
        }
        if let Some(photo_url_value) = msg.photo_url {
            user.photo_url = photo_url_value;
        }

        Ok(diesel::update(schema::users::table)
            .filter(id.eq(&msg.user_id))
            .set(user)
            .get_result::<models::User>(&conn)?)
    }
}

pub struct GetUserById {
    pub user_id: String,
}

impl Message for GetUserById {
    type Result = Result<Option<models::User>>;
}

impl Handler<GetUserById> for DbExecutor {
    type Result = Result<Option<models::User>>;

    fn handle(&mut self, msg: GetUserById, _: &mut Self::Context) -> Self::Result {
        let conn = self.conn()?;
        get_user_by_id(&conn, &msg.user_id)
    }
}
