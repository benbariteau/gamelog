extern crate bcrypt;
extern crate env_logger;
extern crate futures;
extern crate hyper;
extern crate logger;
extern crate params;
extern crate rand;
extern crate router;
extern crate secure_session;
extern crate serde;
extern crate serde_json;
extern crate time;
extern crate tokio_core;
extern crate typemap;

#[macro_use] extern crate askama;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;
#[macro_use] extern crate error_chain;
#[macro_use] extern crate serde_derive;

#[macro_use(itry)] extern crate iron;

use iron::Chain;
use iron::Iron;
use logger::Logger;
use secure_session::middleware::SessionConfig;
use secure_session::middleware::SessionMiddleware;
use secure_session::session::ChaCha20Poly1305SessionManager;
use secure_session::session::SessionManager;

mod handlers;
mod helpers;
mod model;
mod secrets;
mod session;
mod steam;

use handlers::routes;
use session::Session;
use session::SessionKey;
use steam::sync as steam_sync;

mod errors {
    error_chain! { }
}

fn webapp() {
    env_logger::init().unwrap();

    let mut chain = Chain::new(routes());

    chain.link(Logger::new(None));

    // TODO make password configurable
    let session_manager = ChaCha20Poly1305SessionManager::<Session>::from_password(b"foo");
    let session_config = SessionConfig::default();
    chain.link_around(
        SessionMiddleware::<Session, SessionKey, ChaCha20Poly1305SessionManager<Session>>::new(
            session_manager,
            session_config,
        )
    );

    Iron::new(chain).http("0.0.0.0:3000").unwrap();
}

fn main() {
    let arg = std::env::args().nth(1).unwrap();
    match arg.as_str() {
        "webapp" => webapp(),
        "steam-sync" => steam_sync().unwrap(),
        _ => {
            eprintln!("unrecognized argument");
        }
    }
}
