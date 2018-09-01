//! Slog fairing for Rocket.rs
//!
//! This module provides an easy way to use a slog `Logger` in a Rocket application
//!
//! # Usage
//!
//! To your Cargo.toml, add
//!
//!     [dependencies]
//!     rocket-slog = { git = "https://github.com/pwoolcoc/rocket-slog" }
//!
//! In your rocket application, add
//!
//!     extern crate rocket_slog;
//!     use rocket_slog::SlogFairing;
//!
//! Then, when you define your rocket.rs app, you need to do 3 things:
//!
//!   1. Build the slog `Logger` you want to use,
//!   2. Disable Rocket's logging, and
//!   3. Attach the SlogFairing to your application
//!
//! # Example
//!
//! ```rust,ignore
//! #![feature(custom_derive, plugin)]
//! #![plugin(rocket_codegen)]
//!
//! extern crate rocket;
//! extern crate rocket_slog;
//! #[macro_use(slog_o, slog_kv)] extern crate slog;
//! extern crate slog_term;
//! extern crate slog_async;
//!
//! use rocket::Config;
//! use slog::Drain;
//! use rocket_slog::SlogFairing;
//!
//! fn main() {
//!     let decorator = slog_term::TermDecorator::new().build();
//!     let drain = slog_term::FullFormat::new(decorator).build().fuse();
//!     let drain = slog_async::Async::new(drain).build().fuse();
//!     let logger = slog::Logger::root(drain, slog_o!("version" => env!("CARGO_PKG_VERSION")));
//!
//!     let fairing = SlogFairing::new(logger);
//!
//!     let config = Config::development().unwrap();
//!     rocket::custom(config, false) // disables logging
//!         .attach(fairing)
//!         .launch();
//! }
//! ```
//!
//! The fairing also adds your logger to rocket's managed state, so you can do this in your routes
//! too:
//!
//! ```rust,ignore
//! use rocket_slog::SyncLogger;
//!
//! #[get("/")]
//! fn index(log: SyncLogger) -> String {
//!     
//! }
//! ```
#[macro_use] extern crate slog;
extern crate rocket;

use std::sync::Arc;

use slog::Logger;
use rocket::{Data, Request, Response, Rocket};
use rocket::fairing::{Fairing, Info, Kind};

pub type SyncLogger = Arc<Logger>;
pub struct SlogFairing(SyncLogger);

impl SlogFairing {
    pub fn new(root_logger: Logger) -> SlogFairing {
        SlogFairing(Arc::new(root_logger))
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
            slog_info!(&self.0, "config"; "key" => "config_path", "value" => ?config.config_path);
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
        let addr = format!("http://{}:{}", &config.address, &config.port);
        slog_info!(&self.0, "listening"; "address" => %addr);

    }

    fn on_request(&self, request: &mut Request, _: &Data) {
        slog_info!(self.0, "request"; "method" => ?request.method(), "uri" => ?request.uri().as_str());
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
