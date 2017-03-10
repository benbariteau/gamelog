extern crate rusqlite;

use std::path::Path;

pub struct UserGame {
    pub id: u32,
    pub game_id: u32,
    pub user_id: u32,
    pub play_state: String,
    pub start_date: i64,
    pub end_date: i64,
}

pub struct User {
    pub id: u32,
    pub username: String,
}

mod errors {
    error_chain! { }
}

use errors::{Error, ResultExt};

fn get_conn() -> rusqlite::Result<rusqlite::Connection> {
    rusqlite::Connection::open(Path::new("gamelog.db"))
}

pub fn get_user_by_id(user_id: u32) -> Result<User, rusqlite::Error> {
    let conn = try!(get_conn());
    let mut stmt = try!(conn.prepare("SELECT id, username FROM user WHERE id = ?"));

    stmt.query_row(
        &[&user_id],
        |row| {
            User {
                id: row.get(0),
                username: row.get(1),
            }
        },
    )
}

pub fn get_user_by_name(username: String) -> Result<User, rusqlite::Error> {
    let conn = try!(get_conn());
    let mut stmt = try!(conn.prepare("SELECT id, username FROM user WHERE username = ?"));

    stmt.query_row(
        &[&username],
        |row| {
            User {
                id: row.get(0),
                username: row.get(1),
            }
        },
    )
}

pub fn get_user_from_id_or_name(user_string: String) -> Result<User, Error> {
    match user_string.parse::<u32>() {
        Ok(user_id) => get_user_by_id(user_id).chain_err(|| "unable to find user with specified id"),
        Err(_) => get_user_by_name(user_string.to_string()).chain_err(|| "unable to find user with specified username"),
    }
}


pub fn get_user_games(user_id: u32) -> Result<Vec<UserGame>, rusqlite::Error> {
    let conn = try!(get_conn());
    let mut stmt = try!(conn.prepare("SELECT id, game_id, user_id, play_state, start_date, beat_date FROM user_game WHERE user_id = ?"));

    let mut user_games = Vec::new();
    for user_game_result in try!(
        stmt.query_map(
            &[&user_id],
            |row| {
                UserGame {
                    id: row.get(0),
                    game_id: row.get(1),
                    user_id: row.get(2),
                    play_state: row.get(3),
                    start_date: row.get(4),
                    end_date: row.get(5),
                }
            },
        )
    ) {
        user_games.push(try!(user_game_result));
    }
    
    Ok(user_games)
}

pub fn get_user_game_names(user_id: u32) -> Result<Vec<String>, rusqlite::Error> {
    let user_games = try!(get_user_games(user_id));

    let conn = try!(get_conn());
    let mut stmt = try!(
        conn.prepare(
            // put the right number of binds in the IN clause
            format!(
                "SELECT name FROM game WHERE id IN ({})",
                user_games.iter().map(|_| "?").collect::<Vec<&str>>().join(", "),
            ).as_str()
        )
    );

    let mut game_names = Vec::new();
    for game_name_result in try!(stmt.query_map(
        &user_games.iter().map(|user_game| &user_game.id as &rusqlite::types::ToSql).collect::<Vec<&rusqlite::types::ToSql>>()[..],
        |row| row.get(0),
    )) {
        game_names.push(try!(game_name_result));
    }

    Ok(game_names)
}
