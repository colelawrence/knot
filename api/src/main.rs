#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;
#[macro_use]
extern crate redis_async;
#[macro_use]
extern crate serde_derive;

use clap::{App, AppSettings, Arg, SubCommand};

mod logging;

pub mod access;
mod db;
mod mem;
mod server;

pub mod config;
pub use config::Config;

fn main() {
    ::std::env::set_var("RUST_LOG", "actix_web=info,api=info");
    logging::init();

    let default_config = Config::default();

    if let Err(dotenv_error) = dotenv::dotenv() {
        warn!("Unable to process the .env file: {}", dotenv_error);
    }

    let args = App::new("Knot")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("File collection and organization")
        .setting(AppSettings::ArgRequiredElseHelp)
        .arg(
            Arg::with_name("DEBUG")
                .short("d")
                .long("debug")
                .help("Verbose output for troubleshooting"),
        )
        .subcommand(
            SubCommand::with_name("start")
                .about("Starts the Knot Web Application Server")
                .arg(
                    Arg::with_name("PORT")
                        .short("p")
                        .long("port")
                        .value_name("PORT")
                        .help(&format!(
                            "Specify the port to start on [default: {}]",
                            default_config.http_port
                        )),
                )
                .arg(
                    Arg::with_name("HOST")
                        .short("h")
                        .long("host")
                        .value_name("HOST")
                        .help(&format!(
                            "Specify the hostname for the server [default: {}]",
                            default_config.http_host
                        )),
                )
                .arg(
                    Arg::with_name("PUBLIC_URL")
                        .short("u")
                        .long("public-url")
                        .value_name("URL")
                        .help(&format!(
                            "Specify the root public url for the server including protocol scheme [default: {}]",
                            default_config.http_public_url
                        )),
                ),
        )
        .subcommand(
            SubCommand::with_name("show-config").about("Displays the current configuration based on the defaults, environment, and the .env file."),
        )
        .subcommand(
            SubCommand::with_name("debug-config").about("Displays the current configuration and where each variable comes from (default or env)."),
        )
        .get_matches();

    let mut config = default_config.with_environment().expect("Environment variables parsed correctly");

    match args.subcommand_name() {
        Some("start") => {
            // actix::System::new("knot").run();
            let start_args = args.subcommand_matches("start").unwrap();
            config = config.apply_arguments(start_args).expect("Command line arguments parsed correctly");
            server::start::start(config);
        }
        Some("show-config") => {
            println!("{}", config);
        }
        Some("debug-config") => {
            println!("{:?}", config);
        }
        _ => {}
}
}
