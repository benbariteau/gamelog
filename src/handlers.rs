use askama::Template;
use futures::Future;
use futures::Stream;
use hyper;
use iron::IronResult;
use iron::Plugin;
use iron::Request;
use iron::Response;
use iron::headers::ContentType;
use iron::modifiers::RedirectRaw;
use iron::status;
use params::Params;
use params;
use router::Router;
use serde_json;
use time;
use tokio_core;
use typemap;

use errors::Error;
use errors::ResultExt;
use errors;
use model;
use secrets::get_secrets;
use templates;

#[derive(Serialize, Deserialize)]
pub(crate) struct Session {
    pub(crate) user_id: i64
}

pub(crate) struct SessionKey {}

impl typemap::Key for SessionKey {
    type Value = Session;
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

    let user_games_with_names = itry!(model::get_user_games_with_names(user.id));
    let games = user_games_with_names.iter().map(|game_info| {
        let &(ref name, ref game) = game_info;
        templates::GameNameAndPlayState{
            name: name,
            play_state: &game.play_state,
        }
    }).collect();

    let template_context = templates::UserLog {
        _parent: templates::Base{},
        username: user.username,
        games: games,
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
        templates::SignupForm{
            _parent: templates::Base{},
        }.render(),
    ));
    response.headers.set(ContentType::html());

    Ok(response)
}

fn login_form(_: &mut Request) -> IronResult<Response> {
    let mut response = Response::with((
        status::Ok,
        templates::LoginForm{
            _parent: templates::Base{},
        }.render(),
    ));
    response.headers.set(ContentType::html());

    Ok(response)
}

fn get_user_signup_info(req: &mut Request) -> errors::Result<model::UserSignupInfo> {
    let params = req.get_ref::<Params>().chain_err(|| "unable to get params map")?;

    let username = get_param_string_from_param_map(params, "username")?;
    let email = get_param_string_from_param_map(params, "email")?;
    let password = get_param_string_from_param_map(params, "password")?;

    Ok(model::UserSignupInfo{
        username: username,
        email: email,
        password: password,
    })
}

fn signup(req: &mut Request) -> IronResult<Response> {
    let user_signup_info = itry!(get_user_signup_info(req));
    itry!(model::signup(user_signup_info));

    Ok(Response::with((status::SeeOther, RedirectRaw("/".to_string()))))
}

fn get_login_info_from_request(req: &mut Request) -> errors::Result<model::LoginInfo> {
    let params = req.get_ref::<Params>().chain_err(|| "unable to get params map")?;

    let username_or_email = get_param_string_from_param_map(params, "username-or-email")?;
    let password = get_param_string_from_param_map(params, "password")?;

    Ok(model::LoginInfo{
        username_or_email: username_or_email,
        password: password,
    })
}

fn login(req: &mut Request) -> IronResult<Response> {
    let login_info = itry!(get_login_info_from_request(req));
    let user_id = itry!(model::login(login_info));
    req.extensions.insert::<SessionKey>(Session{user_id: user_id});

    Ok(Response::with((status::SeeOther, RedirectRaw("/".to_string()))))
}

fn get_user_from_request(req: &Request) -> Result<model::User, Error> {
    let user_id = req.extensions.get::<SessionKey>().ok_or::<Error>("no session".into())?.user_id;

    model::get_user_by_id(user_id).chain_err(|| "can't get user from database")
}

fn get_param_string_from_param_map(param_map: &params::Map, key: &str) -> errors::Result<String> {
    match param_map.find(
        &[key]
    ).ok_or::<Error>(format!("{} not provided", key).into())? {
        &params::Value::String(ref value) => Ok(value.clone()),
        _ => Err("param isn't a string".into()),
    }
}

fn add_user_game_form(_: &mut Request) -> IronResult<Response> {
    let mut response = Response::with((
        status::Ok,
        templates::AddUserGameForm{
            _parent: templates::Base{},
        }.render(),
    ));
    response.headers.set(ContentType::html());

    Ok(response)
}

fn add_user_game(req: &mut Request) -> IronResult<Response> {
    let user = itry!(get_user_from_request(req));
    let params = itry!(req.get_ref::<Params>().chain_err(|| "unable to get params map"));
    let name = itry!(get_param_string_from_param_map(params, "name"));
    let game_id = itry!(model::upsert_game(name));
    let state = itry!(get_param_string_from_param_map(params, "state"));
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

fn steam_collection(_: &mut Request) -> IronResult<Response> {
    let secrets = itry!(get_secrets());
    let mut core = itry!(tokio_core::reactor::Core::new());
    let client = hyper::Client::new(&core.handle());
    let response_future = client.get(
        itry!(
            format!(
                "http://api.steampowered.com/IPlayerService/GetOwnedGames/v0001/?key={}&steamid={}&format=json",
                secrets.steam_api_key,
                "76561197976392138",
            ).parse()
        )
    ).and_then(|res| res.body().concat2());
    let body = itry!(core.run(response_future));
    let thing: serde_json::Value = itry!(serde_json::from_slice(&body.to_vec()));

    Ok(
        Response::with(
            (status::Ok, itry!(serde_json::to_string(&thing))),
        )
    )
}

fn me(req: &mut Request) -> IronResult<Response> {
    match req.extensions.get::<SessionKey>() {
        Some(session) => Ok(Response::with(
            (status::SeeOther, RedirectRaw(format!("/log/{}", session.user_id)))
        )),
        None => Ok(Response::with(
            (status::SeeOther, RedirectRaw("/login".to_string()))
        )),
    }
}

pub fn routes() -> Router {
    let mut router = Router::new();
    router.get("/", home, "home");
    router.get("/log/:user", user_log, "user_log");
    router.get("/me", me, "me");
    router.get("/signup", signup_form, "signup_form");
    router.post("/signup", signup, "signup");
    router.get("/login", login_form, "login_form");
    router.post("/login", login, "login");
    router.get("/collection/add", add_user_game_form, "add_user_game_form");
    router.post("/collection/add", add_user_game, "add_user_game");
    router.get("/collection/steam", steam_collection, "steam_collection");

    router
}
