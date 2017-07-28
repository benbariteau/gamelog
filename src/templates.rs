use askama::Template;
use std;
use askama;

pub struct GameNameAndPlayState<'a> {
    pub name: &'a String,
    pub play_state: &'a String,
}

#[derive(Template)]
#[template(path = "base.html")]
pub struct Base {}

#[derive(Template)]
#[template(path = "user_log.html")]
pub struct UserLog<'a> {
    pub _parent: Base,
    pub username: String,
    pub games: Vec<GameNameAndPlayState<'a>>,
}

#[derive(Template)]
#[template(path = "signup_form.html")]
pub struct SignupForm {
    pub _parent: Base,
}

#[derive(Template)]
#[template(path = "login_form.html")]
pub struct LoginForm {
    pub _parent: Base,
}

#[derive(Template)]
#[template(path = "add_user_game_form.html")]
pub struct AddUserGameForm {
    pub _parent: Base,
}
