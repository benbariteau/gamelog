use secrets::get_secrets;
use tokio_core;
use hyper;
use serde_json;
use errors;
use errors::ResultExt;
use futures::Future;
use futures::Stream;

pub(crate) fn sync() -> Result<(), errors::Error> {
    let secrets = get_secrets()?;
    let mut core = tokio_core::reactor::Core::new().chain_err(|| "unable to intialize tokio core")?;
    let client = hyper::Client::new(&core.handle());
    let response_future = client.get(
        format!(
            "http://api.steampowered.com/IPlayerService/GetOwnedGames/v0001/?key={}&steamid={}&format=json",
            secrets.steam_api_key,
            "76561197976392138",
        ).parse().chain_err(|| "unable to parse url")?
    ).and_then(|res| res.body().concat2());
    let body = core.run(response_future).chain_err(|| "unable to reap future")?;
    let thing: serde_json::Value = serde_json::from_slice(&body.to_vec()).chain_err(|| "unable to parse json")?;
    println!("{}", serde_json::to_string(&thing).chain_err(|| "unable to dump to json")?);
    Ok(())
}
