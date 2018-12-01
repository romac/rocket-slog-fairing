#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use(debug)] extern crate slog;
extern crate rocket_slog;
extern crate sloggers;

use std::error::Error;
use rocket::config::{Config, Environment, LoggingLevel};
use rocket_slog::{SlogFairing, SyncLogger};
use sloggers::{
    Build,
    terminal::{
        TerminalLoggerBuilder,
        Destination,
    },
    types::Severity,
};

#[get("/")]
fn index(log: SyncLogger) -> &'static str {
    debug!(log, "some log message");
    "Hello world"
}

fn main() -> Result<(), Box<Error>> {
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    builder.destination(Destination::Stderr);
    let logger = builder.build()?;
    let fairing = SlogFairing::new(logger);
    let config = Config::build(Environment::Development)
            .log_level(LoggingLevel::Off) // disables logging
            .finalize()
            .unwrap();
    rocket::custom(config)
        .mount("/", routes![index])
        .attach(fairing)
        .launch();
    Ok(())
}

