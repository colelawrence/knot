use crate::db::{new_pool, DbExecutor};
use crate::mem::MemExecutor;
use actix::prelude::{Addr, SyncArbiter};
use actix_redis::RedisActor;
use actix_web::{http::Method, middleware::Logger, App, HttpRequest};
use std::sync::Arc;

mod google;
mod sessions;

use crate::config::{Config, NotEmpty};

const NUM_DB_THREADS: usize = 4;

pub struct AppState {
    pub db: Addr<DbExecutor>,
    pub mem: MemExecutor,
    pub config: Arc<Config>,
}

fn index(_req: &HttpRequest<AppState>) -> &'static str {
    "Hello world!"
}

pub fn create(config: Config) -> App<AppState> {
    let database_url = config
        .database_url
        .not_empty()
        .expect("DATABASE_URL must be set");
    let database_pool = new_pool(database_url).expect("Failed to create pool.");

    let database_address =
        SyncArbiter::start(NUM_DB_THREADS, move || DbExecutor(database_pool.clone()));

    let redis_addr = RedisActor::start(config.redis_url.clone());
    let mem_executor = MemExecutor::new(redis_addr);

    let state = AppState {
        db: database_address.clone(),
        mem: mem_executor,
        config: Arc::new(config),
    };

    App::with_state(state)
        .middleware(Logger::default())
        .resource("/", |r| r.f(index))
        .scope("/auth", |scope| {
            scope.nested("/v0", |scope| {
                scope
                    .resource("login/session", |r| {
                        r.method(Method::POST)
                            .with_async(sessions::create_login_session);
                        r.method(Method::GET).with(sessions::login_session_i_am)
                    })
                    .resource("login/session/register", |r| {
                        r.method(Method::POST)
                            .with_async(sessions::register_login_session)
                    })
                    .resource("login/session/user", |r| {
                        r.method(Method::POST)
                            .with_async(sessions::create_user_session)
                    })
                    .resource("me", |r| {
                        r.method(Method::GET).with(sessions::user_session_i_am)
                    })
                    .resource("google/login_url", |r| {
                        r.method(Method::POST)
                            .with_async(sessions::create_google_login_url);
                    })
                    .resource("google/callback", |r| {
                        r.method(Method::GET).with_async(sessions::google_callback);
                    })
            })
        })
}
