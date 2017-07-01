extern crate bcrypt;
extern crate env_logger;
extern crate iron_sessionstorage;
extern crate logger;
extern crate params;
extern crate rand;
extern crate router;
extern crate time;
extern crate secure_session;
extern crate typemap;
extern crate serde;

#[macro_use] extern crate askama;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;
#[macro_use] extern crate error_chain;
#[macro_use] extern crate serde_derive;

#[macro_use(itry)] extern crate iron;

use askama::Template;
use iron::Chain;
use iron::Iron;
use iron::IronResult;
use iron::Plugin;
use iron::Request;
use iron::Response;
use iron::headers::ContentType;
use iron::modifiers::RedirectRaw;
use iron::status;
use iron_sessionstorage::SessionRequestExt;
use iron_sessionstorage::SessionStorage;
use iron_sessionstorage::Value;
use iron_sessionstorage::backends::SignedCookieBackend;
use secure_session::session::SessionManager;
use secure_session::session::ChaCha20Poly1305SessionManager;
use secure_session::middleware::SessionMiddleware;
use secure_session::middleware::SessionConfig;
use logger::Logger;
use params::Params;
use router::Router;

use errors::Error;
use errors::ResultExt;

mod model;

mod errors {
    error_chain! { }
}

#[derive(Template)]
#[template(path = "base.html")]
struct BaseTemplate {}

#[derive(Template)]
#[template(path = "user_log.html")]
struct UserLogTemplate {
    _parent: BaseTemplate,
    username: String,
    games: Vec<String>,
}

#[derive(Template)]
#[template(path = "signup_form.html")]
struct SignupFormTemplate {
    _parent: BaseTemplate,
}

#[derive(Template)]
#[template(path = "login_form.html")]
struct LoginFormTemplate {
    _parent: BaseTemplate,
}


#[derive(Template)]
#[template(path = "add_user_game_form.html")]
struct AddUserGameFormTemplate {
    _parent: BaseTemplate,
}

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

    let user = match user_string.parse::<i64>() {
        Ok(user_id) => itry!(model::get_user_by_id(user_id)),
        Err(_) => {
            let user = itry!(model::get_user_by_name(user_string.to_string()));
            return Ok(Response::with((status::SeeOther, RedirectRaw(format!("/log/{}", user.id)))))
        }
    };

    let template_context = UserLogTemplate {
        _parent: BaseTemplate{},
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
        SignupFormTemplate{
            _parent: BaseTemplate{},
        }.render(),
    ));
    response.headers.set(ContentType::html());

    Ok(response)
}

fn login_form(_: &mut Request) -> IronResult<Response> {
    let mut response = Response::with((
        status::Ok,
        LoginFormTemplate{
            _parent: BaseTemplate{},
        }.render(),
    ));
    response.headers.set(ContentType::html());

    Ok(response)
}

fn get_username_and_password_from_request(req: &mut Request) -> errors::Result<(String, String)> {
    let params = req.get_ref::<Params>().chain_err(|| "unable to get params map")?;

    let username = get_param_string_from_param_map(params, "username".to_string())?;
    let password = get_param_string_from_param_map(params, "password".to_string())?;

    Ok((username, password))
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
    let user_id = itry!(model::login(username.clone(), password));
    req.extensions.insert::<SessionKey>(Session{user_id: user_id});

    Ok(Response::with((status::SeeOther, RedirectRaw("/".to_string()))))
}

fn get_user_from_request(req: &mut Request) -> Result<model::User, Error> {
    let user_id = req.extensions.get::<SessionKey>().ok_or::<Error>("no session".into())?.user_id;

    model::get_user_by_id(user_id).chain_err(|| "can't get user from database")
}

fn get_param_string_from_param_map(param_map: &params::Map, key: String) -> errors::Result<String> {
    match param_map.find(
        &[key.as_str()]
    ).ok_or::<Error>(format!("{} not provided", key).into())? {
        &params::Value::String(ref value) => Ok(value.clone()),
        _ => Err("param isn't a string".into()),
    }
}

fn add_user_game_form(_: &mut Request) -> IronResult<Response> {
    let mut response = Response::with((
        status::Ok,
        AddUserGameFormTemplate{
            _parent: BaseTemplate{},
        }.render(),
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
        model::add_user_game(model::NewUserGame{
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

#[derive(Serialize, Deserialize)]
struct Session {
    user_id: i64
}

struct SessionKey {}

impl typemap::Key for SessionKey {
    type Value = Session;
}

fn main() {
    env_logger::init().unwrap();

    let mut router = Router::new();
    router.get("/", home, "home");
    router.get("/log/:user", user_log, "user_log");
    router.get("/signup", signup_form, "signup_form");
    router.post("/signup", signup, "signup");
    router.get("/login", login_form, "login_form");
    router.post("/login", login, "login");
    router.get("/collection/add", add_user_game_form, "add_user_game_form");
    router.post("/collection/add", add_user_game, "add_user_game");

    let mut chain = Chain::new(router);

    chain.link(Logger::new(None));

    chain.link_around(SessionStorage::new(SignedCookieBackend::new(vec![1, 2, 3, 4])));

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
