use serde_json;

use errors;
use errors::ResultExt;
use std::fs::File;

#[derive(Serialize, Deserialize)]
pub(crate) struct Secrets {
    pub(crate) steam_api_key: String
}

pub(crate) fn get_secrets() -> Result<Secrets, errors::Error> {
    let fd = File::open("secrets.json").chain_err(|| "unable to open secrets.json")?;
    serde_json::from_reader(fd).chain_err(|| "unable to read secrets from file")
}
