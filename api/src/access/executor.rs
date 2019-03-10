use actix::prelude::*;

use crate::db::DbExecutor;
use crate::mem::MemExecutor;

/// Controls database access
pub struct AccessExecutor {
    pub mem: Addr<MemExecutor>,
    pub db: Addr<DbExecutor>,
    pub settings: AccessSettings,
}

pub struct AccessSettings {
    pub public_url: String,
    pub google_client_id: String,
    pub google_client_secret: String,
}

impl Actor for AccessExecutor {
    type Context = Context<Self>;
}
