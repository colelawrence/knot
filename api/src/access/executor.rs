use actix::prelude::*;

use std::sync::Arc;

use crate::db::DbExecutor;
use crate::mem::MemExecutor;

/// Controls database access
pub struct AccessExecutor {
    pub mem: Addr<MemExecutor>,
    pub db: Addr<DbExecutor>,
    pub settings: Arc<AccessSettings>,
}

pub struct AccessSettings {
    pub google_login_domain: Option<String>,
    pub google_callback_uri: String,
    pub google_client_id: String,
    pub google_client_secret: String,
}

impl Actor for AccessExecutor {
    type Context = Context<Self>;
}
