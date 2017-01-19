extern crate iron;
extern crate router;

use iron::prelude::*;
use iron::status;
use router::Router;

fn home(_: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::Ok, "Hello world!")))
}

fn main() {
    let mut router = Router::new();
    router.get("/", home, "home");

    Iron::new(router).http("0.0.0.0:3000").unwrap();
}
