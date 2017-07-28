use iron::Request;
use iron::prelude::*;
use params::Params;
use params;

use errors::Error;
use errors::ResultExt;
use errors;
use model;
use session::Session;
use session::SessionKey;

pub fn get_user_signup_info(req: &mut Request) -> Result<model::UserSignupInfo, Error> {
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

pub fn get_user_from_session(session: &Session) -> Result<model::User, Error> {
    model::get_user_by_id(session.user_id).chain_err(|| "can't get user from database")
}

pub fn get_user_from_request(req: &Request) -> Result<model::User, Error> {
    let session = req.extensions.get::<SessionKey>().ok_or::<Error>("no session".into())?;

    get_user_from_session(session)
}

pub fn get_param_string_from_param_map(param_map: &params::Map, key: &str) -> errors::Result<String> {
    match param_map.find(
        &[key]
    ).ok_or::<Error>(format!("{} not provided", key).into())? {
        &params::Value::String(ref value) => Ok(value.clone()),
        _ => Err("param isn't a string".into()),
    }
}

