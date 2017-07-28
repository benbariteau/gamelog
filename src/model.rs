use std::fmt::Write;

use bcrypt;
use diesel::ExpressionMethods;
use diesel::connection::Connection;
use diesel::prelude::ExecuteDsl;
use diesel::prelude::FilterDsl;
use diesel::prelude::LimitDsl;
use diesel::prelude::LoadDsl;
use diesel::prelude::OrderDsl;
use diesel::result::OptionalExtension;
use diesel::sqlite::SqliteConnection;
use diesel;
use rand::OsRng;
use rand::Rng;

use self::errors::Error;
use self::errors::ResultExt;
use self::schema::game;
use self::schema::user;
use self::schema::user_game;
use self::schema::user_private;


mod errors {
    error_chain! { }
}

mod schema {
    table! {
        user {
            id -> BigInt,
            username -> VarChar,
            email -> VarChar,
        }
    }
    table! {
        user_private {
            id -> BigInt,
            user_id -> BigInt,
            password_hash -> VarChar,
            salt -> VarChar,
        }
    }
    table! {
        game {
            id -> BigInt,
            name -> VarChar,
            steam_id -> Nullable<BigInt>,
        }
    }
    table! {
        user_game {
            id -> BigInt,
            game_id -> BigInt,
            user_id -> BigInt,
            play_state -> VarChar,
            acquisition_date -> BigInt,
            start_date -> Nullable<BigInt>,
            beat_date -> Nullable<BigInt>,
        }
    }
}

#[derive(Queryable)]
pub struct UserGame {
    pub id: i64,
    pub game_id: i64,
    pub user_id: i64,
    pub play_state: String,
    pub acquisition_date: i64,
    pub start_date: Option<i64>,
    pub beat_date: Option<i64>,
}

#[derive(Insertable)]
#[table_name="user_game"]
pub struct NewUserGame {
    pub game_id: i64,
    pub user_id: i64,
    pub play_state: String,
    pub acquisition_date: i64,
    pub start_date: Option<i64>,
    pub beat_date: Option<i64>,
}

#[derive(Insertable)]
#[table_name="user"]
struct NewUser {
    username: String,
    email: String,
}

#[derive(Insertable)]
#[table_name="user_private"]
struct NewUserPrivate {
    user_id: i64,
    password_hash: String,
    salt: String,
}

#[derive(Queryable)]
pub struct UserPrivate {
    pub id: i64,
    pub user_id: i64,
    pub password_hash: String,
    pub salt: String,
}

#[derive(Queryable)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
}

#[derive(Queryable)]
pub struct Game {
    pub id: i64,
    pub name: String,
    // TODO make this into a u64
    pub steam_id: Option<i64>,
}

#[derive(Insertable)]
#[table_name="game"]
pub struct NewGame {
    pub name: String,
    pub steam_id: Option<i64>,
}

fn get_diesel_conn() -> Result<SqliteConnection, Error> {
    SqliteConnection::establish("gamelog.db").chain_err(|| "unable to get sqlite connection")
}

pub fn get_user_by_id(user_id: i64) -> Result<User, Error> {
    let conn = get_diesel_conn()?;
    user::table.filter(
        user::id.eq(user_id)
    ).get_result::<User>(&conn).chain_err(|| "unable to load user")
}

pub fn get_user_by_name(username: String) -> Result<User, Error> {
    let conn = get_diesel_conn()?;
    user::table.filter(
        user::username.eq(username)
    ).get_result::<User>(&conn).chain_err(|| "unable to load user")
}

pub fn get_user_games(user_id: i64) -> Result<Vec<UserGame>, Error> {
    let conn = get_diesel_conn().chain_err(|| "unable to get db connection")?;
    schema::user_game::table.filter(
        schema::user_game::user_id.eq(user_id),
    ).load::<UserGame>(&conn).chain_err(|| "unable to load user games")
}

pub fn get_user_game_names(user_id: i64) -> Result<Vec<String>, Error> {
    let user_games = get_user_games(user_id)?;

    let game_ids: Vec<i64> = user_games.iter().map(|user_game| user_game.game_id).collect();
    let conn = get_diesel_conn()?;
    let games = game::table.filter(
        game::id.eq_any(game_ids),
    ).load::<Game>(
        &conn,
    ).chain_err(|| "unable to get game names")?;
    let game_names = games.iter().map(|game| game.name.clone()).collect();

    Ok(game_names)
}


pub struct UserSignupInfo{
    pub username: String,
    pub email: String,
    pub password: String,
}


pub fn signup(user_signup_info: UserSignupInfo) -> Result<(), Error> {
    let mut rng = OsRng::new().chain_err(|| "unable to create rng")?;
    let mut salt_bytes: Vec<u8> = vec![0; 16];
    rng.fill_bytes(&mut salt_bytes);
    let mut salt = String::new();
    for byte in salt_bytes.iter() {
        write!(&mut salt, "{:X}", byte).unwrap();
    }

    let salted_password = format!("{}{}", user_signup_info.password, salt);
    let password_hash_result = bcrypt::hash(
        salted_password.as_str(),
        bcrypt::DEFAULT_COST,
    );
    let password_hash: String = password_hash_result.chain_err(|| "unable to hash password")?;

    let conn = get_diesel_conn()?;
    let new_user = NewUser{
        username: user_signup_info.username,
        email: user_signup_info.email,
    };

    conn.transaction(|| {
        diesel::insert(
            &new_user,
        ).into(
            user::table,
        ).execute(&conn)?;
        
        let user_new = user::table.order(
            user::id.desc(),
        ).limit(1).get_result::<User>(&conn)?;

        let new_user_private = NewUserPrivate{
            user_id: user_new.id,
            password_hash: password_hash,
            salt: salt,
        };

        diesel::insert(
            &new_user_private,
        ).into(
            user_private::table,
        ).execute(&conn)
    }).chain_err(|| "unable to add new user")?;
    Ok(())
}

pub struct LoginInfo {
    pub username_or_email: String,
    pub password: String,
}

fn get_user_from_email(email: String) -> Result<User, Error> {
    let conn = get_diesel_conn()?;
    user::table.filter(
        user::email.eq(&email),
    ).get_result::<User>(
        &conn
    ).chain_err(|| {format!("user with email '{}' not found", &email)})
}

fn get_user_from_username_or_email(username_or_email: String) -> Result<User, Error> {
    let conn = get_diesel_conn()?;
    let user_row_result = user::table.filter(
        user::username.eq(&username_or_email),
    ).get_result::<User>(
        &conn
    );

    match user_row_result {
        Ok(user_row) => Ok(user_row),
        Err(error) => {
            match error {
                diesel::result::Error::NotFound => {
                    get_user_from_email(username_or_email)
                },
                _ => Err(
                    Error::with_chain(
                        error,
                        format!("user with username '{}' not found", &username_or_email),
                    )
                ),
            }
        }
    }
}

pub fn login(login_info: LoginInfo) -> Result<i64, Error> {
    let conn = get_diesel_conn()?;
    let user_row = get_user_from_username_or_email(login_info.username_or_email)?;

    let user_private_row = user_private::table.filter(
        user_private::user_id.eq(user_row.id)
    ).get_result::<UserPrivate>(
        &conn
    ).chain_err(|| "unable to load user_private row")?;

    let salted_password = format!("{}{}", login_info.password, user_private_row.salt);

    if bcrypt::verify(
        salted_password.as_str(),
        user_private_row.password_hash.as_str(),
    ).chain_err(|| "error while verifying hashed password")? {
        Ok(user_row.id)
    } else {
        Err("password does not match".into())
    }
}

fn get_game_by_name_with_conn(
    name: &String,
    conn: &SqliteConnection,
) -> Result<Game, diesel::result::Error> {
    game::table.filter(
        game::name.eq(name),
    ).get_result::<Game>(
        conn,
    )
}

fn get_optional_game_by_name(name: &String) -> Result<Option<Game>, Error> {
    let conn = get_diesel_conn()?;
    get_game_by_name_with_conn(name, &conn).optional().chain_err(|| "unable to load game")
}

fn get_game_by_name(name: &String) -> Result<Game, Error> {
    let conn = get_diesel_conn()?;
    get_game_by_name_with_conn(name, &conn).chain_err(|| "unable to load game")
}

pub fn get_game_by_steam_id(steam_id: u64) -> Result<Game, Error> {
    game::table.filter(
        game::steam_id.eq(steam_id as i64),
    ).get_result::<Game>(
        &get_diesel_conn()?,
    ).chain_err(|| "can't load game")
}

pub fn insert_game(game: NewGame) -> Result<i64, Error> {
    let conn = get_diesel_conn()?;

    diesel::insert(
        &game,
    ).into(
        game::table,
    ).execute(
        &conn
    ).chain_err(|| "unable to insert new game")?;

    Ok(get_game_by_name(&game.name)?.id)
}

pub fn upsert_game(name: String) -> Result<i64, Error> {
    match get_optional_game_by_name(&name)? {
        Some(game_row) => {
            return Ok(game_row.id)
        },
        None => {},
    }
    insert_game(
        NewGame{
            name: name.clone(),
            steam_id: None,
        }
    )
}

pub fn add_user_game(user_game: NewUserGame) -> Result<(), Error> {
    let conn = get_diesel_conn()?;
    diesel::insert(
        &user_game,
    ).into(
        user_game::table,
    ).execute(
        &conn,
    ).chain_err(|| "unable to save new user game")?;
    Ok(())
}
