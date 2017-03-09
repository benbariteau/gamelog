#[macro_use(itry)]
extern crate iron;
extern crate router;
extern crate urlencoded;
extern crate logger;
extern crate env_logger;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate askama;

use iron::prelude::*;
use iron::Chain;
use iron::status;
use router::Router;
use askama::Template;
use logger::Logger;

mod model;

mod errors {
    error_chain! {}
}

use errors::Error;

#[derive(Template)]
#[template(path = "user_log.html")]
struct UserLogTemplate {
    username: String,
    games: Vec<String>,
}

fn home(_: &mut Request) -> IronResult<Response> {
    let user_games: Vec<String> = itry!(model::get_user_games(1)).iter().map(
        |user_game| user_game.id.to_string(),
    ).collect();
    Ok(Response::with((status::Ok, user_games.join(" "))))
}

fn user_log(req: &mut Request) -> IronResult<Response> {
    let params = itry!(
        req.extensions.get::<Router>().ok_or::<Error>(
            "no router".into()
        )
    );

    let user_id: u32 = itry!(
        itry!(
            params.find("user").ok_or::<Error>(
                "no user id provided".into()
            )
        ).parse()
    );
    let user = itry!(model::get_user(user_id));
    let user_games = itry!(model::get_user_games(user_id));
    let template_context = UserLogTemplate {
        username: user.username,
        games: Vec::new(),
    };
    Ok(Response::with((status::Ok, template_context.render())))
}

fn main() {
    env_logger::init().unwrap();

    let mut router = Router::new();
    router.get("/", home, "home");
    router.get("/log/:user", user_log, "user_log");

    let mut chain = Chain::new(router);

    let (logger_before, logger_after) = Logger::new(None);
    chain.link_before(logger_before);
    chain.link_after(logger_after);

    Iron::new(chain).http("0.0.0.0:3000").unwrap();
}
