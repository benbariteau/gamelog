#[macro_use(itry)]
extern crate iron;
extern crate router;
extern crate logger;
extern crate env_logger;
extern crate params;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate askama;
extern crate rusqlite;
extern crate rand;
extern crate bcrypt;

use iron::prelude::*;
use iron::headers::ContentType;
use iron::Chain;
use iron::modifiers::RedirectRaw;
use iron::status;
use router::Router;
use askama::Template;
use logger::Logger;
use params::{Params, Value};

mod errors {
    error_chain! { }
}

use errors::Error;

mod model;

#[derive(Template)]
#[template(path = "user_log.html")]
struct UserLogTemplate {
    username: String,
    games: Vec<String>,
}

#[derive(Template)]
#[template(path = "signup_form.html")]
struct SignupFormTemplate {}

fn home(_: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::Ok, "Welcome!")))
}

fn user_log(req: &mut Request) -> IronResult<Response> {
    let params = itry!(
        req.extensions.get::<Router>().ok_or::<Error>(
            "no router".into()
        )
    );

    let user_string = itry!(
        params.find("user").ok_or::<Error>(
            "no user id or username provided".into()
        )
    );

    let user = itry!(model::get_user_from_id_or_name(user_string.to_string()), (status::NotFound, "User not found"));

    let template_context = UserLogTemplate {
        username: user.username,
        games: itry!(model::get_user_game_names(user.id)),
    };

    let mut response = Response::with((
        status::Ok,
        template_context.render(),
    ));

    response.headers.set(ContentType::html());

    Ok(response)
}

fn signup_form(_: &mut Request) -> IronResult<Response> {
    let mut response = Response::with((
        status::Ok,
        SignupFormTemplate{}.render(),
    ));
    response.headers.set(ContentType::html());

    Ok(response)
}

fn signup(req: &mut Request) -> IronResult<Response> {
    let params = itry!(req.get_ref::<Params>());
    let username_result: Result<&String, Error> = match itry!(params.find(&["username"]).ok_or::<Error>("no username provided".into())) {
        &Value::String(ref username) => Ok(username),
        _ => Err("username isn't a string".into()),
    };
    let username = itry!(username_result);
    let password_result: Result<&String, Error> = match itry!(params.find(&["password"]).ok_or::<Error>("no password provided".into())) {
        &Value::String(ref password) => Ok(password),
        _ => Err("password isn't a string".into()),
    };
    let password = itry!(password_result);
    itry!(model::signup(username, password));

    Ok(Response::with((status::SeeOther, RedirectRaw("/".to_string()))))
}

fn main() {
    env_logger::init().unwrap();

    let mut router = Router::new();
    router.get("/", home, "home");
    router.get("/log/:user", user_log, "user_log");
    router.get("/signup", signup_form, "signup_form");
    router.post("/signup", signup, "signup");

    let mut chain = Chain::new(router);

    let (logger_before, logger_after) = Logger::new(None);
    chain.link_before(logger_before);
    chain.link_after(logger_after);

    Iron::new(chain).http("0.0.0.0:3000").unwrap();
}
