use std::sync::Arc;

use actix::Addr;
use crate::access::AccessExecutor;
use crate::Config;

pub struct State {
    pub access: Addr<AccessExecutor>,
    pub config: Arc<Config>,
}
