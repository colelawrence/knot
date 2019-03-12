#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;
#[macro_use]
extern crate redis_async;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate failure;

mod app;
mod auth;
mod config;
mod db;
mod error;
mod mem;
mod prelude;
mod utils;

use config::{Config, NotEmpty};

fn main() {
    kankyo::load().expect("Error loading .env file");

    if std::env::var("RUST_LOG").ok().is_none() {
        std::env::set_var("RUST_LOG", "auth=debug,actix_web=info");
    }
    env_logger::init();

    let sys = actix::System::new("auth");

    let config = Config::default().with_environment();

    let bind_address = config
        .http_bind_address
        .not_empty()
        .expect("HTTP_BIND_ADDRESS is not set");
    let public_url = config.http_public_url.clone();

    let mut server = actix_web::server::new(move || app::create(config.clone()));

    // Autoreload with systemfd & listenfd
    // from: https://actix.rs/docs/autoreload/
    use listenfd::ListenFd;
    let mut listenfd = ListenFd::from_env();
    server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        server.listen(l)
    } else {
        server
            .bind(&bind_address)
            .unwrap_or_else(|_| panic!("Could not bind server to address {}", &bind_address))
    };

    println!("You can access the server at {}", bind_address);
    println!("                             {}", public_url);

    server.run();
}
