//! An HTTP client that passes cookies back and forth between our clients and pickleballtournaments.com

use std::convert::{Infallible, TryFrom};
use std::str::FromStr;
use std::sync::Arc;

use http::Error as HttpError;
use reqwest::cookie::{CookieStore, Jar};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, IntoHeaderName};
use reqwest::redirect::Policy;
use reqwest::{
    Client as ReqwestClient, Error, IntoUrl, RequestBuilder as ReqwestRequestBuilder, Response, Url,
};
use rocket::http::{Cookie, CookieJar};
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::Serialize;

pub struct Client<'r> {
    outgoing_cookies: Option<&'r CookieJar<'r>>,
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

    #[allow(dead_code)]
    pub fn post<U>(&self, url: U) -> RequestBuilder
    where
        U: IntoUrl,
    {
        let request = self.client.post(url);

        RequestBuilder {
            client: self,
            request,
        }
    }
}

pub struct ClientBuilder<'r> {
    cookies: Option<&'r CookieJar<'r>>,
    incoming_cookies_url: Option<Url>,
    default_headers: HeaderMap,
    redirect_policy: Option<Policy>,
}

impl<'r> ClientBuilder<'r> {
    pub fn new() -> Self {
        let default_headers = std::array::IntoIter::new([
            ("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"),
            ("Accept-Language", "en-US,en;q=0.5"),
            ("Connection", "keep-alive"),
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

        Self {
            cookies: None,
            incoming_cookies_url: None,
            default_headers,
            redirect_policy: None,
        }
    }

    pub fn forward_cookies(mut self, cookies: &'r CookieJar<'r>, from_url: Url) -> Self {
        self.cookies = Some(cookies);
        self.incoming_cookies_url = Some(from_url);
        self
    }

    pub fn default_header<K>(mut self, key: K, value: &str) -> Self
    where
        K: IntoHeaderName,
    {
        self.default_headers.insert(key, value.parse().unwrap());
        self
    }

    #[allow(dead_code)]
    pub fn redirect_policy(mut self, policy: Policy) -> Self {
        self.redirect_policy = Some(policy);
        self
    }

    pub fn build(self) -> Client<'r> {
        let mut client_builder = ReqwestClient::builder().default_headers(self.default_headers);

        let client_cookies = Arc::new(Jar::default());
        if let Some(cookies) = self.cookies {
            let cookie_values = cookies
                .iter()
                .map(|c| HeaderValue::from_str(&c.to_string()).unwrap())
                .collect::<Vec<_>>();
            client_cookies.set_cookies(
                &mut cookie_values.iter(),
                self.incoming_cookies_url.as_ref().unwrap(),
            );
        }
        client_builder = client_builder.cookie_provider(client_cookies);

        if let Some(policy) = self.redirect_policy {
            client_builder = client_builder.redirect(policy)
        }

        Client {
            outgoing_cookies: self.cookies,
            client: client_builder.build().unwrap(),
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

    #[allow(dead_code)]
    pub fn form<T>(self, form: &T) -> Self
    where
        T: Serialize + ?Sized,
    {
        Self {
            client: self.client,
            request: self.request.form(form),
        }
    }

    pub async fn send(self) -> Result<Response, Error> {
        let response = self.request.send().await;

        if let Ok(ref response) = response {
            if let Some(outgoing_cookies) = self.client.outgoing_cookies {
                // Copy any new cookies into the outgoing jar
                for header in response.headers().get_all("set-cookie").iter() {
                    outgoing_cookies
                        .add(Cookie::parse(String::from(header.to_str().unwrap())).unwrap());
                }
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

        let client = ClientBuilder::new()
            .forward_cookies(
                incoming_cookies,
                Url::parse("https://www.pickleballtournaments.com").unwrap(),
            )
            .default_header("Host", "www.pickleballtournaments.com")
            .build();

        Outcome::Success(client)
    }
}
