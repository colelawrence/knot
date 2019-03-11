use askama::Template; // bring trait in scope

use actix::{Actor, SyncArbiter};
use actix_redis::{RedisActor, RedisSessionBackend};
use actix_web::middleware::session::{RequestSession, SessionStorage};
use actix_web::{http, Error};
use actix_web::{middleware, server, App, HttpRequest, HttpResponse};
use futures::Future;

use std::sync::Arc;

use crate::access::{AccessExecutor, AccessSettings};
use crate::db::DbExecutor;
use crate::mem::MemExecutor;
use crate::server::State;
use crate::Config;

use super::login_routes;

pub fn start(config: Config) {
    // let sys = actix::System::new("dewey");
    let config = Arc::new(config);

    // r2d2 pool
    let manager = diesel::r2d2::ConnectionManager::new(&config.database_url);
    let pool = diesel::r2d2::Pool::new(manager).unwrap();

    // Start db executor actors
    let db_addr = SyncArbiter::start(3, move || DbExecutor(pool.clone()));

    let redis_addr = RedisActor::start(&config.redis_url);
    let mem_executor = MemExecutor(redis_addr);
    let mem_addr = mem_executor.start();

    let access_executor = AccessExecutor {
        mem: mem_addr.clone(),
        db: db_addr.clone(),
        settings: Arc::new(AccessSettings {
            google_login_domain: None,
            google_callback_uri: format!("{}/login/google/callback", config.http_public_url),
            google_client_id: (&config.google_oauth_client_id).into(),
            google_client_secret: (&config.google_oauth_client_secret).into(),
        }),
    };

    let access_addr = access_executor.start();

    use listenfd::ListenFd;
    let mut listenfd = ListenFd::from_env();
    let mut server = server::new(move || {
        vec![
            App::new()
                .prefix("/static")
                .handler(
                    "/",
                    actix_web::fs::StaticFiles::new("./static")
                        .unwrap()
                        .show_files_listing(),
                )
                .boxed(),
            App::with_state(State {
                access: access_addr.clone(),
                config: config.clone(),
            })
            .middleware(middleware::Logger::new(r#"%T "%r" %s %b "%{Referer}i""#))
            .middleware(SessionStorage::new(
                RedisSessionBackend::new(&config.redis_url, &[0; 32])
                    .cookie_secure(true) // cookies require https
                    .cookie_name("sess"),
            ))
            .scope("/api", |scope: actix_web::Scope<State>| {
                scope.nested("/v0", |scope: actix_web::Scope<State>| {
                    // scope.nested("/upload", upload::upload_scope)
                    scope
                })
            })
            .scope("/login", login_routes::login_scope)
            // .resource("/", |r| r.f(index))
            .boxed(),
        ]
    });

    // Autoreload with systemfd & listenfd
    // from: https://actix.rs/docs/autoreload/
    server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        server.listen(l)
    } else {
        server.bind("127.0.0.1:8088").unwrap()
    };

    info!("Started http server: 127.0.0.1:8088");
    info!("                     {}", dotenv!("HTTP_HOST"));
    server.run();
}
