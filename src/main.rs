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
extern crate iron_sessionstorage;
extern crate time;

use iron::{Request, Response, Iron, Plugin, IronResult};
use iron::headers::ContentType;
use iron::Chain;
use iron::modifiers::RedirectRaw;
use iron::status;
use router::Router;
use askama::Template;
use logger::Logger;
use params::Params;
use iron_sessionstorage::{SessionStorage, SessionRequestExt, Value};
use iron_sessionstorage::backends::SignedCookieBackend;

use errors::{Error, ResultExt};
mod model;

mod errors {
    error_chain! { }
}


#[derive(Template)]
#[template(path = "user_log.html")]
struct UserLogTemplate {
    username: String,
    games: Vec<String>,
}

#[derive(Template)]
#[template(path = "signup_form.html")]
struct SignupFormTemplate {}

#[derive(Template)]
#[template(path = "add_user_game_form.html")]
struct AddUserGameFormTemplate {}

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

fn get_username_and_password_from_request(req: &mut Request) -> errors::Result<(String, String)> {
    let params = try!(req.get_ref::<Params>().chain_err(|| "unable to get params map"));
    let username_result: Result<&String, Error> = match try!(
        params.find(&["username"]).ok_or::<Error>(
            "no username provided".into()
        )
    ) {
        &params::Value::String(ref username) => Ok(username),
        _ => Err("username isn't a string".into()),
    };
    let username = try!(username_result);
    let password_result: Result<&String, Error> = match try!(
        params.find(&["password"]).ok_or::<Error>(
            "no password provided".into()
        )
    ) {
        &params::Value::String(ref password) => Ok(password),
        _ => Err("password isn't a string".into()),
    };
    let password = try!(password_result);

    Ok((username.clone(), password.clone()))
}

fn signup(req: &mut Request) -> IronResult<Response> {
    let (username, password) = itry!(get_username_and_password_from_request(req));
    itry!(model::signup(username, password));

    Ok(Response::with((status::SeeOther, RedirectRaw("/".to_string()))))
}

struct UserSession {
    username: String,
}

impl Value for UserSession {
    fn get_key() -> &'static str { "user" }
    fn into_raw(self) -> String { self.username }
    fn from_raw(value: String) -> Option<Self> {
        Some(UserSession{username: value})
    }
}

fn login(req: &mut Request) -> IronResult<Response> {
    let (username, password) = itry!(get_username_and_password_from_request(req));
    itry!(model::login(username.clone(), password));
    try!(req.session().set(UserSession{username: username}));

    Ok(Response::with((status::SeeOther, RedirectRaw("/".to_string()))))
}

fn get_user_from_request(req: &mut Request) -> Result<model::User, Error> {
    let user_session_result = match req.session().get::<UserSession>() {
        Ok(user_session) => Ok(user_session),
        Err(_) => Err("unable to get user session"),
    };
    let user_session: Option<UserSession> = try!(user_session_result);
    let username = try!(
        user_session.ok_or::<Error>("no session".into())
    ).username;

    model::get_user_by_name(username.clone()).chain_err(|| "can't get use from db")
}

fn user_profile_self(req: &mut Request) -> IronResult<Response> {
    let user = itry!(get_user_from_request(req));

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

fn get_param_string_from_param_map(param_map: &params::Map, key: String) -> errors::Result<String> {
    match try!(
        param_map.find(&[key.as_str()]).ok_or::<Error>(
            format!("{} not provided", key).into()
        )
    ) {
        &params::Value::String(ref value) => Ok(value.clone()),
        _ => Err("param isn't a string".into()),
    }
}

fn add_user_game_form(_: &mut Request) -> IronResult<Response> {
    let mut response = Response::with((
        status::Ok,
        AddUserGameFormTemplate{}.render(),
    ));
    response.headers.set(ContentType::html());

    Ok(response)
}

fn add_user_game(req: &mut Request) -> IronResult<Response> {
    let user = itry!(get_user_from_request(req));
    let params = itry!(req.get_ref::<Params>().chain_err(|| "unable to get params map"));
    let name = itry!(get_param_string_from_param_map(params, "name".to_string()));
    let game_id = itry!(model::upsert_game(name));
    let state = itry!(get_param_string_from_param_map(params, "state".to_string()));
    itry!(
        model::add_user_game(model::UserGame{
            id: 0,
            game_id: game_id,
            user_id: user.id,
            play_state: state,
            acquisition_date: time::get_time().sec,
            start_date: None,
            beat_date: None,
        })
    );

    Ok(Response::with((status::SeeOther, RedirectRaw("/me".to_string()))))
}

fn main() {
    env_logger::init().unwrap();

    let mut router = Router::new();
    router.get("/", home, "home");
    router.get("/log/:user", user_log, "user_log");
    router.get("/signup", signup_form, "signup_form");
    router.post("/signup", signup, "signup");
    // signup and login are the same right now, the different URI allows them to act differently
    // (POST to different locations)
    router.get("/login", signup_form, "login_form");
    router.post("/login", login, "login");
    router.get("/me", user_profile_self, "user_profile_self");
    router.get("/collection/add", add_user_game_form, "add_user_game_form");
    router.post("/collection/add", add_user_game, "add_user_game");

    let mut chain = Chain::new(router);
    chain.link(Logger::new(None));
    chain.link_around(SessionStorage::new(SignedCookieBackend::new(vec![1, 2, 3, 4])));

    Iron::new(chain).http("0.0.0.0:3000").unwrap();
}
