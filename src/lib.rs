//! Slog fairing for Rocket.rs
//!
//! This module provides an easy way to use a slog `Logger` in a Rocket application
//!
//! # Usage
//!
//! To your Cargo.toml, add
//!
//! ```ignore
//! [dependencies]
//! rocket-slog = "0.4"
//! ```
//!
//! In your rocket application, add
//!
//! ```
//! extern crate rocket_slog;
//! use rocket_slog::SlogFairing;
//! ```
//!
//! Then, when you define your rocket.rs app, you need to do 3 things:
//!
//!   1. Build the slog `Logger` you want to use,
//!   2. Disable Rocket's logging, and
//!   3. Attach the SlogFairing to your application
//!
//! # Example
//!
//! ```rust,no_run
//! #![feature(proc_macro_hygiene, decl_macro)]
//! #[macro_use] extern crate rocket;
//! extern crate rocket_slog;
//! #[macro_use(debug)] extern crate slog;
//! extern crate sloggers;
//!
//! use std::error::Error;
//!
//! use rocket::config::{Config, Environment, LoggingLevel};
//! use rocket_slog::SlogFairing;
//! use sloggers::{
//!     Build,
//!     terminal::{
//!         TerminalLoggerBuilder,
//!         Destination,
//!     },
//!     types::Severity,
//! };
//!
//! fn main() -> Result<(), Box<Error>> {
//!     let mut builder = TerminalLoggerBuilder::new();
//!     builder.level(Severity::Debug);
//!     builder.destination(Destination::Stderr);
//!     let logger = builder.build()?;
//!     let fairing = SlogFairing::new(logger);
//!
//!     let config = Config::build(Environment::Development)
//!             .log_level(LoggingLevel::Off) // disables logging
//!             .finalize()
//!             .unwrap();
//!     rocket::custom(config) 
//!         .attach(fairing)
//!         .launch();
//!     Ok(())
//! }
//! ```
//!
//! The fairing also adds your logger to rocket's managed state, so you can do this in your routes
//! too:
//!
//! ```rust,no_run
//! #![feature(proc_macro_hygiene, decl_macro)]
//! #[macro_use] extern crate rocket;
//! # extern crate rocket_slog;
//! # #[macro_use(debug)] extern crate slog;
//! # extern crate sloggers;
//! # use std::error::Error;
//! # use rocket::config::{Config, Environment, LoggingLevel};
//! # use rocket_slog::SlogFairing;
//! # use sloggers::{
//! #     Build,
//! #     terminal::{
//! #         TerminalLoggerBuilder,
//! #         Destination,
//! #     },
//! #     types::Severity,
//! # };
//! use rocket_slog::SyncLogger;
//!
//! #[get("/")]
//! fn index(log: SyncLogger) -> &'static str {
//!     debug!(log, "some log message");
//!     "Hello world"
//! }
//! # fn main() -> Result<(), Box<Error>> {
//! #     let mut builder = TerminalLoggerBuilder::new();
//! #     builder.level(Severity::Debug);
//! #     builder.destination(Destination::Stderr);
//! #     let logger = builder.build()?;
//! #     let fairing = SlogFairing::new(logger);
//! #     let config = Config::build(Environment::Development)
//! #             .log_level(LoggingLevel::Off) // disables logging
//! #             .finalize()
//! #             .unwrap();
//! #     rocket::custom(config)
//! #         .mount("/", routes![index])
//! #         .attach(fairing)
//! #         .launch();
//! #     Ok(())
//! # }
//! ```
#[macro_use] extern crate slog;
extern crate rocket;

use std::sync::Arc;

use slog::Logger;
use rocket::{Data, Request, Response, Rocket, State};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::request;

/// Newtype struct wrapper around the passed-in slog::Logger
#[derive(Debug, Clone)]
pub struct SyncLogger(Arc<Logger>);

/// Fairing used to provide a rocket.rs application with a slog::Logger
#[derive(Debug, Clone)]
pub struct SlogFairing(SyncLogger);

impl SlogFairing {
    /// Create a new SlogFairing using the slog::Logger
    pub fn new(root_logger: Logger) -> SlogFairing {
        SlogFairing(SyncLogger(Arc::new(root_logger)))
    }
}

impl SyncLogger {
    pub fn get(&self) -> &Logger {
        &*self.0
    }
}

impl std::ops::Deref for SyncLogger {
    type Target = Logger;

    fn deref(&self) -> &Logger {
        &*self.0
    }
}

impl<'a, 'r> request::FromRequest<'a, 'r> for SyncLogger {
    type Error = ();

    fn from_request(req: &'a request::Request<'r>) -> request::Outcome<SyncLogger, ()> {
        let sync_logger = req.guard::<State<SyncLogger>>()?;
        rocket::Outcome::Success(sync_logger.clone())
    }
}

impl Fairing for SlogFairing {
    fn info(&self) -> Info {
        Info {
            name: "Slog Fairing",
            kind: Kind::Attach | Kind::Launch | Kind::Request | Kind::Response,
        }
    }

    fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        {
            let config = rocket.config();
            slog_info!(&self.0, "config"; "key" => "environment", "value" => ?config.environment);
            slog_info!(&self.0, "config"; "key" => "address", "value" => %config.address);
            slog_info!(&self.0, "config"; "key" => "port", "value" => %config.port);
            slog_info!(&self.0, "config"; "key" => "workers", "value" => %config.workers);
            slog_info!(&self.0, "config"; "key" => "log_level", "value" => ?config.log_level);
            // not great, could there be a way to enumerate limits like we do for extras?
            if let Some(forms) = config.limits.get("forms") {
                slog_info!(&self.0, "config"; "key" => "forms limit", "value" => ?forms);
            }
            if let Some(json) = config.limits.get("json") {
                slog_info!(&self.0, "config"; "key" => "json limit", "value" => ?json);
            }
            if let Some(msgpack) = config.limits.get("msgpack") {
                slog_info!(&self.0, "config"; "key" => "msgpack limit", "value" => ?msgpack);
            }
            for (key, val) in &config.extras {
                slog_info!(&self.0, "config"; "key" => &key, "value" => ?val);
            }
        }
        // add managed logger so the user can use it in guards
        Ok(rocket.manage(self.0.clone()))
    }

    fn on_launch(&self, rocket: &Rocket) {
        for route in rocket.routes() {
            if route.rank < 0 {
                slog_info!(&self.0, "route"; "base" => %route.base(), "path" => %route.uri, "method" => %route.method);
            } else {
                slog_info!(&self.0, "route"; "base" => %route.base(), "path" => %route.uri, "rank" => %route.rank);
            }
        }
        // can't seem to get the list of Catchers?

        let config = rocket.config();
        let scheme = if config.tls_enabled() { "https" } else { "http" };
        let addr = format!("{}://{}:{}", &scheme, &config.address, &config.port);
        slog_info!(&self.0, "listening"; "address" => %addr);

    }

    fn on_request(&self, request: &mut Request, _: &Data) {
        slog_info!(self.0, "request"; "method" => ?request.method(), "uri" => ?request.uri().to_string());
    }

    fn on_response(&self, request: &Request, response: &mut Response) {
        let status = response.status();
        let status = format!("{} {}", status.code, status.reason);
        if let Some(ref route) = request.route() {
            slog_info!(&self.0, "response"; "route" => %route, "status" => %status);
        } else {
            slog_info!(&self.0, "response"; "status" => %status);
        }
    }
}
