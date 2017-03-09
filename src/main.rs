#[macro_use(itry)]
extern crate iron;
extern crate router;
extern crate urlencoded;
#[macro_use]
extern crate error_chain;

use iron::prelude::*;
use iron::status;
use router::Router;

mod model;

mod errors {
    error_chain! {}
}

use errors::Error;

fn home(_: &mut Request) -> IronResult<Response> {
    let user_games: Vec<String> = itry!(model::get_user_games(1)).iter().map(
        |user_game| user_game.id.to_string(),
    ).collect();
    Ok(Response::with((status::Ok, user_games.join(" "))))
}

fn user_log(req: &mut Request) -> IronResult<Response> {
    let maybe_router: Result<&router::Params, Error> = req.extensions.get::<Router>().ok_or::<Error>(
        "no router".into()
    );
    let router = itry!(maybe_router);
    let user_id_str = itry!(
        router.find("user").ok_or::<Error>(
            "no user id provided".into()
        )
    );
    let user_id = itry!(user_id_str.parse::<u32>());
    let user_games: Vec<String> = itry!(model::get_user_games(user_id)).iter().map(
        |user_game| user_game.id.to_string(),
    ).collect();

    Ok(Response::with((status::Ok, user_games.join(" "))))
}

fn main() {
    let mut router = Router::new();
    router.get("/", home, "home");
    router.get("/log/:user", user_log, "user_log");

    Iron::new(router).http("0.0.0.0:3000").unwrap();
}
