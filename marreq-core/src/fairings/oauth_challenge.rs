use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Request, Response};
use std::sync::Mutex;

pub(crate) struct OAuthChallenge(pub Mutex<Option<String>>);

pub(crate) fn set_oauth_challenge(request: &Request<'_>, value: String) {
    let challenge = request.local_cache(|| OAuthChallenge(Mutex::new(None)));
    if let Ok(mut current) = challenge.0.lock() {
        *current = Some(value);
    }
}

pub struct OAuthChallengeFairing;

#[rocket::async_trait]
impl Fairing for OAuthChallengeFairing {
    fn info(&self) -> Info {
        Info {
            name: "OAuth Bearer challenge",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let challenge = request.local_cache(|| OAuthChallenge(Mutex::new(None)));
        if let Ok(current) = challenge.0.lock() {
            if let Some(value) = current.as_ref() {
                response.set_header(Header::new("WWW-Authenticate", value.clone()));
            }
        }
    }
}
