use rusqlite;
use bcrypt;
use std::path::Path;
use rand::OsRng;
use rand::Rng;
use std::fmt::Write;
use diesel::insert;
use diesel::sqlite::SqliteConnection;
use diesel::connection::Connection;
use diesel::prelude::{FilterDsl,LoadDsl,ExecuteDsl};
use diesel::ExpressionMethods;

mod schema {
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

use self::schema::user_game;

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

pub struct User {
    pub id: u64,
    pub username: String,
}

mod errors {
    error_chain! { }
}

use errors::{Error, ResultExt};

fn get_diesel_conn() -> Result<SqliteConnection, Error> {
    SqliteConnection::establish("gamelog.db").chain_err(|| "unable to get sqlite connection")
}

fn get_conn() -> rusqlite::Result<rusqlite::Connection> {
    rusqlite::Connection::open(Path::new("gamelog.db"))
}

pub fn get_user_by_id(user_id: u64) -> Result<User, rusqlite::Error> {
    let conn = get_conn()?;
    let mut stmt = conn.prepare("SELECT id, username FROM user WHERE id = ?")?;

    stmt.query_row(
        &[&(user_id as i64)],
        |row| {
            let id: i64 = row.get(1);
            User {
                id: id as u64,
                username: row.get(1),
            }
        },
    )
}

pub fn get_user_by_name(username: String) -> Result<User, rusqlite::Error> {
    let conn = get_conn()?;
    let mut stmt = conn.prepare("SELECT id, username FROM user WHERE username = ?")?;

    stmt.query_row(
        &[&username],
        |row| {
            let id: i64 = row.get(0);
            User {
                id: id as u64,
                username: row.get(1),
            }
        },
    )
}

pub fn get_user_from_id_or_name(user_string: String) -> Result<User, Error> {
    match user_string.parse::<u64>() {
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

pub fn get_user_game_names(user_id: u64) -> Result<Vec<String>, Error> {
    let user_games = get_user_games(user_id as i64).chain_err(|| "unable to get user_games")?;

    let conn = get_conn().chain_err(|| "unable to get db connection")?;
    let mut stmt = conn.prepare(
        // put the right number of binds in the IN clause
        format!(
            "SELECT name FROM game WHERE id IN ({})",
            user_games.iter().map(|_| "?").collect::<Vec<&str>>().join(", "),
        ).as_str()
    ).chain_err(|| "unable to load game rows")?;
    let user_ids: Vec<i64> = user_games.iter().map(|user_game| user_game.id as i64).collect();
    let mut game_names = Vec::new();
    for game_name_result in stmt.query_map(
        &user_ids.iter().map(|id| id as &rusqlite::types::ToSql).collect::<Vec<&rusqlite::types::ToSql>>()[..],
        |row| row.get(0),
    ).chain_err(|| "unable to get user_game rows")? {
        game_names.push(game_name_result.chain_err(|| "unable to get game name")?);
    }

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

    let conn = get_conn().chain_err(|| "unable to get db connection")?;
    conn.execute(
        "INSERT INTO user (username, password_hash, salt) values (?, ?, ?)",
        &[&username, &password_hash, &salt],
    ).chain_err(|| "unable to insert user row")?;
    Ok(())
}

pub fn login(username: String, password: String) -> Result<u64, Error> {
    let conn = get_conn().chain_err(|| "unable to get db connection")?;
    let mut stmt = conn.prepare("SELECT id, password_hash, salt FROM user WHERE username = ?").chain_err(|| "unable to prepare statement")?;

    let (id, password_hash, salt): (u64, String, String) = stmt.query_row(
        &[&username],
        |row| {
            let id: i64 = row.get(0);
            (id as u64, row.get(1), row.get(2))
        },
    ).chain_err(|| "unable to get user stuff")?;

    let salted_password = format!("{}{}", password, salt);

    if bcrypt::verify(
        salted_password.as_str(),
        password_hash.as_str(),
    ).chain_err(|| "error while verifying hashed password")? {
        Ok(id)
    } else {
        Err("password does not match".into())
    }
}

pub fn upsert_game(name: String) -> Result<u64, Error> {
    let conn = get_conn().chain_err(|| "unable to get db connection")?;
    let mut stmt = conn.prepare("SELECT id FROM game WHERE name = ?").chain_err(|| "unable to prepare statement")?;
    let mut mapped_rows = stmt.query_map(
        &[&name],
        |row| {
            let id: i64 = row.get(0);
            id as u64
        }
    ).chain_err(|| "unable to get game id")?;

    match mapped_rows.nth(0) {
        Some(result) => {
            return result.chain_err(|| "error mapping rows")
        },
        None => {},
    }

    let mut stmt = conn.prepare("INSERT INTO game (name) values (?)").chain_err(|| "unable to prepare statement")?;
    stmt.insert(
        &[&name],
    ).chain_err(|| "unable to insert game row").map(
        |game_id| game_id as u64
    )
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
