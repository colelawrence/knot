use actix::prelude::*;
use actix_redis::RedisActor;


/// This is memory executor actor. We are going to run 3 of them in parallel.
pub struct MemExecutor(pub Addr<RedisActor>);

impl Actor for MemExecutor {
    type Context = SyncContext<Self>;
}
