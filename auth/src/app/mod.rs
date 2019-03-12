use crate::db::{new_pool, DbExecutor};
use crate::mem::MemExecutor;
use actix::prelude::{Addr, SyncArbiter};
use actix_redis::RedisActor;
use actix_web::{
    http::{header, Method},
    middleware::Logger,
    App, HttpRequest,
};
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
                        r.method(Method::GET)
                            .with_async(sessions::login_session_i_am)
                    })
                    .resource("login/session/register", |r| {
                        r.method(Method::POST)
                            .with_async(sessions::register_login_session)
                    })
                    .resource("me", |r| {
                        r.method(Method::GET)
                            .with_async(sessions::user_session_i_am)
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
    // .scope("/api", |scope| {
    //     scope
    //         // User routes ↓
    //         .resource("users", |r| {
    //             r.method(Method::POST).with_async(users::register)
    //         })
    //         .resource("users/login", |r| {
    //             r.method(Method::POST).with_async(users::login)
    //         })
    //         .resource("user", |r| {
    //             r.method(Method::GET).with_async(users::get_current);
    //             r.method(Method::PUT).with_async(users::update)
    //         })
    //         // Profile routes ↓
    //         .resource("profiles/{username}", |r| {
    //             r.method(Method::GET).with_async(profiles::get)
    //         })
    //         .resource("profiles/{username}/follow", |r| {
    //             r.method(Method::POST).with_async(profiles::follow);
    //             r.method(Method::DELETE).with_async(profiles::unfollow)
    //         })
    //         // Article routes ↓
    //         .resource("articles", |r| {
    //             r.method(Method::GET).with_async(articles::list);
    //             r.method(Method::POST).with_async(articles::create)
    //         })
    //         .resource("articles/feed", |r| {
    //             r.method(Method::GET).with_async(articles::feed)
    //         })
    //         .resource("articles/{slug}", |r| {
    //             r.method(Method::GET).with_async(articles::get);
    //             r.method(Method::PUT).with_async(articles::update);
    //             r.method(Method::DELETE).with_async(articles::delete)
    //         })
    //         .resource("articles/{slug}/favorite", |r| {
    //             r.method(Method::POST).with_async(articles::favorite);
    //             r.method(Method::DELETE).with_async(articles::unfavorite)
    //         })
    //         .resource("articles/{slug}/comments", |r| {
    //             r.method(Method::GET).with_async(articles::comments::list);
    //             r.method(Method::POST).with_async(articles::comments::add)
    //         })
    //         .resource("articles/{slug}/comments/{comment_id}", |r| {
    //             r.method(Method::DELETE)
    //                 .with_async(articles::comments::delete)
    //         })
    //         // Tags routes ↓
    //         .resource("tags", |r| r.method(Method::GET).with_async(tags::get))
    // })
}
