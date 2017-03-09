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

fn get_conn() -> rusqlite::Result<rusqlite::Connection> {
    rusqlite::Connection::open(Path::new("gamelog.db"))
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
