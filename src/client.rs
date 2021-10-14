//! An HTTP client that passes cookies back and forth between our clients and pickleballtournaments.com

use std::convert::Infallible;
use std::str::FromStr;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use reqwest::{Client as HttpClient, IntoUrl, Response};
use rocket::http::{Cookie, CookieJar};
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome, Request};

pub struct Client<'r> {
    cookies: &'r CookieJar<'r>,
    client: HttpClient,
}

impl<'r> Client<'r> {
    pub async fn get<U>(&self, url: U) -> Response
    where
        U: IntoUrl,
    {
        let response = self.client.get(url).send().await.unwrap();

        // Copy any new cookies into the client's jar
        for header in response.headers().get_all("set-cookie").iter() {
            self.cookies
                .add(Cookie::parse(String::from(header.to_str().unwrap())).unwrap());
        }

        response
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Client<'r> {
    type Error = Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let incoming_cookies = try_outcome!(request.guard::<&CookieJar<'_>>().await);

        let incoming_cookie_values = incoming_cookies
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(";");

        let default_headers = std::array::IntoIter::new([
            ("Accept-Language", "en-US,en;q=0.5"),
            ("Cookie", &incoming_cookie_values),
            ("Upgrade-Insecure-Requests", "1"),
            (
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:93.0) Gecko/20100101 Firefox/93.0",
            ),
        ])
        .map(|(k, v)| {
            (
                HeaderName::from_str(k).unwrap(),
                HeaderValue::from_str(v).unwrap(),
            )
        })
        .collect::<HeaderMap<_>>();

        let client = HttpClient::builder()
            .default_headers(default_headers.clone())
            .redirect(Policy::none())
            .build()
            .unwrap();

        Outcome::Success(Client {
            cookies: incoming_cookies,
            client,
        })
    }
}
