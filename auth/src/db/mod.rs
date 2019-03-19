pub mod models;
mod schema;
pub mod users;
use crate::prelude::*;

use actix::prelude::{Actor, SyncContext};
use diesel::{
    pg::PgConnection,
    r2d2::{self, ConnectionManager, Pool, PooledConnection},
};

pub type Conn = PgConnection;
pub type PgPool = Pool<ConnectionManager<Conn>>;
pub type PooledConn = PooledConnection<ConnectionManager<Conn>>;

pub struct DbExecutor(pub PgPool);

impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

impl DbExecutor {
    pub fn conn(&self) -> Result<PooledConn> {
        Ok(self.0.get()?)
    }
}

pub fn new_pool<S: Into<String>>(database_url: S) -> Result<PgPool> {
    let manager = ConnectionManager::<Conn>::new(database_url.into());
    let pool = r2d2::Pool::builder().build(manager)?;
    Ok(pool)
}

use crate::app::AppState;
use actix::Addr;
use actix_web::{FromRequest, HttpRequest};

impl FromRequest<AppState> for Addr<DbExecutor> {
    type Config = ();
    type Result = Addr<DbExecutor>;

    #[inline]
    fn from_request(req: &HttpRequest<AppState>, _cfg: &Self::Config) -> Self::Result {
        req.state().db.clone()
    }
}
