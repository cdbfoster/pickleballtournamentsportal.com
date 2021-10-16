//! An HTTP client that passes cookies back and forth between our clients and pickleballtournaments.com

use std::convert::{Infallible, TryFrom};
use std::str::FromStr;
use std::sync::Arc;

use http::Error as HttpError;
use reqwest::cookie::{CookieStore, Jar};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use reqwest::{
    Client as ReqwestClient, Error, IntoUrl, RequestBuilder as ReqwestRequestBuilder, Response, Url,
};
use rocket::http::{Cookie, CookieJar};
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome, Request};

pub struct Client<'r> {
    outgoing_cookies: &'r CookieJar<'r>,
    client: ReqwestClient,
}

impl<'r> Client<'r> {
    pub fn get<U>(&self, url: U) -> RequestBuilder
    where
        U: IntoUrl,
    {
        let request = self.client.get(url);

        RequestBuilder {
            client: self,
            request,
        }
    }
}

pub struct RequestBuilder<'r> {
    client: &'r Client<'r>,
    request: ReqwestRequestBuilder,
}

impl<'r> RequestBuilder<'r> {
    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<HttpError>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<HttpError>,
    {
        Self {
            client: self.client,
            request: self.request.header(key, value),
        }
    }

    pub async fn send(self) -> Result<Response, Error> {
        let response = self.request.send().await;

        if let Ok(ref response) = response {
            // Copy any new cookies into the outgoing jar
            for header in response.headers().get_all("set-cookie").iter() {
                self.client
                    .outgoing_cookies
                    .add(Cookie::parse(String::from(header.to_str().unwrap())).unwrap());
            }
        }

        response
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Client<'r> {
    type Error = Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let incoming_cookies = try_outcome!(request.guard::<&CookieJar<'_>>().await);

        let client_cookies = Arc::new(Jar::default());

        let url = Url::parse("https://www.pickleballtournaments.com").unwrap();
        let cookie_values = incoming_cookies
            .iter()
            .map(|c| HeaderValue::from_str(&c.to_string()).unwrap())
            .collect::<Vec<_>>();
        client_cookies.set_cookies(&mut cookie_values.iter(), &url);

        let default_headers = std::array::IntoIter::new([
            ("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"),
            ("Accept-Encoding", "gzip, deflate, br"),
            ("Accept-Language", "en-US,en;q=0.5"),
            ("Connection", "keep-alive"),
            ("Host", "www.pickleballtournaments.com"),
            ("Sec-Fetch-Dest", "document"),
            ("Sec-Fetch-Mode", "navigate"),
            ("Sec-Fetch-Site", "none"),
            ("Sec-Fetch-User", "?1"),
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

        let client = ReqwestClient::builder()
            .default_headers(default_headers.clone())
            .cookie_provider(client_cookies)
            .redirect(Policy::none())
            .build()
            .unwrap();

        Outcome::Success(Client {
            outgoing_cookies: incoming_cookies,
            client,
        })
    }
}
