use actix::prelude::*;
use actix_redis::RedisActor;

/// This is memory executor actor
pub struct MemExecutor {
    pub redis: Addr<RedisActor>,
}

impl Actor for MemExecutor {
    type Context = Context<Self>;
}

impl MemExecutor {
    pub fn conn(&self) -> &Addr<RedisActor> {
        &self.redis
    }
}
