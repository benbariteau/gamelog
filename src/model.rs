use bcrypt;
use diesel;
use rand::OsRng;
use rand::Rng;
use std::fmt::Write;
use diesel::insert;
use diesel::result::OptionalExtension;
use diesel::sqlite::SqliteConnection;
use diesel::connection::Connection;
use diesel::prelude::{FilterDsl,LoadDsl,ExecuteDsl,OrderDsl,LimitDsl};
use diesel::ExpressionMethods;

mod schema {
    table! {
        user {
            id -> BigInt,
            username -> VarChar,
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

use self::schema::{user_game,user,game,user_private};

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
}

#[derive(Queryable)]
pub struct Game {
    pub id: i64,
    pub name: String,
}

#[derive(Insertable)]
#[table_name="game"]
struct NewGame {
    name: String,
}

mod errors {
    error_chain! { }
}

use errors::{Error, ResultExt};

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

pub fn get_user_from_id_or_name(user_string: String) -> Result<User, Error> {
    match user_string.parse::<i64>() {
        Ok(user_id) => get_user_by_id(user_id).chain_err(|| "unable to find user with specified id"),
        Err(_) => get_user_by_name(user_string.to_string()).chain_err(|| "unable to find user with specified username"),
    }
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

pub fn signup(username: String, password: String) -> Result<(), Error> {
    let mut rng = OsRng::new().chain_err(|| "unable to create rng")?;
    let mut salt_bytes: Vec<u8> = vec![0; 16];
    rng.fill_bytes(&mut salt_bytes);
    let mut salt = String::new();
    for byte in salt_bytes.iter() {
        write!(&mut salt, "{:X}", byte).unwrap();
    }

    let salted_password = format!("{}{}", password, salt);
    let password_hash_result = bcrypt::hash(
        salted_password.as_str(),
        bcrypt::DEFAULT_COST,
    );
    let password_hash: String = password_hash_result.chain_err(|| "unable to hash password")?;

    let conn = get_diesel_conn()?;
    let new_user = NewUser{username: username};
    conn.transaction(|| {
        insert(
            &new_user,
        ).into(
            user::table,
        ).execute(&conn)?;
        
        let user_new =user::table.order(
            user::id.desc(),
        ).limit(1).get_result::<User>(&conn)?;

        let new_user_private = NewUserPrivate{
            user_id: user_new.id,
            password_hash: password_hash,
            salt: salt,
        };

        insert(
            &new_user_private,
        ).into(
            user_private::table,
        ).execute(&conn)
    }).chain_err(|| "unable to add new user")?;
    Ok(())
}

pub fn login(username: String, password: String) -> Result<i64, Error> {
    let conn = get_diesel_conn()?;
    let user_row = user::table.filter(
        user::username.eq(&username),
    ).get_result::<User>(
        &conn
    ).chain_err(|| {format!("user with username '{}' not found", &username)})?;

    let user_private_row = user_private::table.filter(
        user_private::user_id.eq(user_row.id)
    ).get_result::<UserPrivate>(
        &conn
    ).chain_err(|| "unable to load user_private row")?;

    let salted_password = format!("{}{}", password, user_private_row.salt);

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

pub fn upsert_game(name: String) -> Result<i64, Error> {
    match get_optional_game_by_name(&name)? {
        Some(game_row) => {
            return Ok(game_row.id)
        },
        None => {},
    }

    let conn = get_diesel_conn()?;

    insert(
        &NewGame{
            name: name.clone(),
        },
    ).into(
        game::table,
    ).execute(
        &conn
    ).chain_err(|| "unable to insert new game")?;

    Ok(get_game_by_name(&name)?.id)
}

pub fn add_user_game(user_game: NewUserGame) -> Result<(), Error> {
    let conn = get_diesel_conn()?;
    insert(
        &user_game,
    ).into(
        user_game::table,
    ).execute(
        &conn,
    ).chain_err(|| "unable to save new user game")?;
    Ok(())
}
