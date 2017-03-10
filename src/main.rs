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
use iron::headers::ContentType;
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
    Ok(Response::with((status::Ok "Welcome!")))
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

    let template_context = UserLogTemplate {
        username: itry!(model::get_user(user_id)).username,
        games: itry!(model::get_user_game_names(user_id)),
    };

    let mut response = Response::with((
        status::Ok,
        template_context.render(),
    ));

    response.headers.set(ContentType::html());

    Ok(response)
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
