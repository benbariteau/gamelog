use typemap;

#[derive(Serialize, Deserialize)]
pub struct Session {
    pub user_id: i64
}

pub struct SessionKey {}

impl typemap::Key for SessionKey {
    type Value = Session;
}
