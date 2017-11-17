use askama::Template;
use iron::IronResult;
use iron::Plugin;
use iron::Request;
use iron::Response;
use iron::headers::ContentType;
use iron::modifiers::RedirectRaw;
use iron::status;
use params::Params;
use router::Router;
use time;

use errors::Error;
use errors::ResultExt;
use errors;
use helpers::get_param_string_from_param_map;
use helpers::get_user_from_session;
use helpers::get_user_signup_info;
use model;
use session::Session;
use session::SessionKey;
use serde_json;
use std::fs::File;
use std::collections::HashMap;


macro_rules! try_session {
    ( $req : expr ) => (
        match $req.extensions.get::<SessionKey>() {
            Some(session) => session,
            None => return Ok(Response::with(
                (status::SeeOther, RedirectRaw("/login".to_string()))
            )),
        }
    );
}

macro_rules! redirect_logged_out_user {
    ( $req : expr ) => (
        {
            let _ = try_session!($req);
        }
    )
}


#[derive(Template)]
#[template(path = "base.html")]
struct BaseTemplate {
    logged_in: bool,
}

#[derive(Template)]
#[template(path = "user_log.html")]
struct UserLogTemplate {
    _parent: BaseTemplate,
    username: String,
    games: Vec<UserGamePresenter>,
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
#[template(path = "user_game_form.html")]
struct UserGameFormTemplate<'a> {
    _parent: BaseTemplate,
    page_title: String,
    submit_button: String,
    user_game_states: Vec<UserGameState<'a>>,
    platforms: Vec<Platform>,
    name: String,
    set_user_game_state: String,
    set_platform: String,
}

#[derive(Template)]
#[template(path = "user_settings_form.html")]
struct UserSettingsFormTemplate {
    _parent: BaseTemplate,
    username: String,
    steam_id: String,
}

struct UserGamePresenter {
    name: String,
    user_game: model::UserGame,
}

struct UserGameState<'a> {
    display: &'a str,
    value: &'a str,
}

#[derive(Serialize, Deserialize)]
struct Platform {
    name: String,
    short_name: String,
    slug: String,
}

fn user_game_states<'a>() -> Vec<UserGameState<'a>> {
    vec![
        UserGameState{
            display: "Unplayed",
            value: "unplayed",
        },
        UserGameState{
            display: "Unfinished",
            value: "unfinished",
        },
        UserGameState{
            display: "Beaten",
            value: "beaten",
        },
        UserGameState{
            display: "Completed",
            value: "completed",
        },
        UserGameState{
            display: "100%",
            value: "100_percent",
        },
        UserGameState{
            display: "Won't Beat",
            value: "wont_beat",
        },
        UserGameState{
            display: "Multiplayer",
            value: "multiplayer",
        },
        UserGameState{
            display: "Null",
            value: "null",
        },
    ]
}

fn home(req: &mut Request) -> IronResult<Response> {
    let logged_in = req.extensions.get::<SessionKey>().is_some();
    let mut response = Response::with((
        status::Ok,
        itry!(BaseTemplate{logged_in: logged_in}.render()),
    ));

    response.headers.set(ContentType::html());

    Ok(response)
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
    let games = user_games_with_names.into_iter().map(|(name, game)| UserGamePresenter{
        name: name,
        user_game: game,
    }).collect();

    let template_context = UserLogTemplate {
        _parent: BaseTemplate{
            logged_in: req.extensions.get::<SessionKey>().is_some(),
        },
        username: user.username,
        games: games,
    };

    let mut response = Response::with((
        status::Ok,
        itry!(template_context.render()),
    ));

    response.headers.set(ContentType::html());

    Ok(response)
}

fn signup_form(req: &mut Request) -> IronResult<Response> {
    let mut response = Response::with((
        status::Ok,
        itry!(SignupFormTemplate{
            _parent: BaseTemplate{
                logged_in: req.extensions.get::<SessionKey>().is_some(),
            },
        }.render()),
    ));
    response.headers.set(ContentType::html());

    Ok(response)
}

fn login_form(req: &mut Request) -> IronResult<Response> {
    let mut response = Response::with((
        status::Ok,
        itry!(LoginFormTemplate{
            _parent: BaseTemplate{
                logged_in: req.extensions.get::<SessionKey>().is_some(),
            },
        }.render()),
    ));
    response.headers.set(ContentType::html());

    Ok(response)
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

fn logout(req: &mut Request) -> IronResult<Response> {
    req.extensions.remove::<SessionKey>();

    Ok(Response::with((status::SeeOther, RedirectRaw("/".to_string()))))
}

fn add_user_game_form(req: &mut Request) -> IronResult<Response> {
    redirect_logged_out_user!(req);

    let mut response = Response::with((
        status::Ok,
        itry!(UserGameFormTemplate{
            _parent: BaseTemplate{
                logged_in: req.extensions.get::<SessionKey>().is_some(),
            },
            page_title: "Add a Game".to_string(),
            submit_button: "Add Game".to_string(),
            user_game_states: user_game_states(),
            platforms: itry!(get_platforms()),
            name: "".to_string(),
            set_user_game_state: "".to_string(),
            set_platform: "".to_string(),
        }.render()),
    ));
    response.headers.set(ContentType::html());

    Ok(response)
}

fn add_user_game(req: &mut Request) -> IronResult<Response> {
    let user = {
        let session = try_session!(req);
        itry!(get_user_from_session(session))
    };

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

fn me(req: &mut Request) -> IronResult<Response> {
    let session = try_session!(req);
    Ok(Response::with(
        (status::SeeOther, RedirectRaw(format!("/log/{}", session.user_id)))
    ))
}

fn user_settings_form(req: &mut Request) -> IronResult<Response> {
    redirect_logged_out_user!(req);

    let user = {
        let session = try_session!(req);
        itry!(get_user_from_session(session))
    };

    let steam_id = match user.steam_id {
        Some(id) => id,
        None => "".to_string(),
    };

    let mut response = Response::with((
        status::Ok,
        itry!(UserSettingsFormTemplate{
            _parent: BaseTemplate{
                logged_in: req.extensions.get::<SessionKey>().is_some(),
            },
            username: user.username,
            steam_id: steam_id,
        }.render()),
    ));
    response.headers.set(ContentType::html());

    Ok(response)
}

fn get_user_settings_from_request(req: &mut Request) -> errors::Result<(String, String)> {
    let params = req.get_ref::<Params>().chain_err(|| "unable to get params map")?;

    let username = get_param_string_from_param_map(params, "username")?;
    let steam_id = get_param_string_from_param_map(params, "steam_id")?;

    Ok((username, steam_id))
}

fn user_settings_update(req: &mut Request) -> IronResult<Response> {
    let (username, steam_id_raw) = { itry!(get_user_settings_from_request(req)) };
    let steam_id = if steam_id_raw == "" { None } else { Some(steam_id_raw) };
    let session = try_session!(req);
    itry!(model::update_username(session.user_id, username));
    itry!(model::update_steam_id(session.user_id, steam_id));

    Ok(Response::with((status::SeeOther, RedirectRaw("/settings".to_string()))))
}

fn get_platforms() -> Result<Vec<Platform>, Error> {
    let platform_config = File::open("config/platforms.json").chain_err(|| "unable to read platforms config")?;
    let manufacturer_to_platforms: HashMap<String, Vec<Platform>> = serde_json::from_reader(platform_config).chain_err(|| "unable to parse platforms config")?;
    Ok(manufacturer_to_platforms.into_iter().flat_map(|(_, platforms)| platforms.into_iter()).collect())
}

fn edit_user_game_form(req: &mut Request) -> IronResult<Response> {
    let session = try_session!(req);

    let params = itry!(
        req.extensions.get::<Router>().ok_or::<Error>("no router".into())
    );

    let user_game_id_string = itry!(
        params.find("user_game_id").ok_or::<Error>("no user game id provided".into())
    );

    let user_game_id: i64 = itry!(user_game_id_string.parse());

    let user_game = itry!(model::get_user_game_by_id(user_game_id));

    if user_game.user_id != session.user_id {
        return Ok(Response::with((status::Forbidden, "You don't own this game!")))
    }

    let game = itry!(model::get_game_by_id(user_game.game_id));

    let mut response = Response::with((
        status::Ok,
        itry!(UserGameFormTemplate{
            _parent: BaseTemplate{logged_in: true},
            page_title: format!("Edit Game: {}", game.name),
            submit_button: "Update Game".to_string(),
            user_game_states: user_game_states(),
            platforms: itry!(get_platforms()),
            name: game.name,
            set_user_game_state: user_game.play_state,
            set_platform: user_game.platform,
        }.render()),
    ));
    response.headers.set(ContentType::html());

    Ok(response)
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
    router.get("/collection/edit/:user_game_id", edit_user_game_form, "edit_user_game_form");
    router.get("/settings", user_settings_form, "user_settings_form");
    router.post("/settings", user_settings_update, "user_settings_update");
    router.get("/logout", logout, "logout");

    router
}
