#[macro_use(itry)]
extern crate iron;
extern crate router;

use iron::prelude::*;
use iron::status;
use router::Router;

mod model;

fn home(_: &mut Request) -> IronResult<Response> {
    let user_games: Vec<String> = itry!(model::get_user_games(1)).iter().map(
        |user_game| user_game.id.to_string(),
    ).collect();
    Ok(Response::with((status::Ok, user_games.join(" "))))
}

fn main() {
    let mut router = Router::new();
    router.get("/", home, "home");

    Iron::new(router).http("0.0.0.0:3000").unwrap();
}
