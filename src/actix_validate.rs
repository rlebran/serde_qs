//! Functionality for using `serde_qs` with `actix_web`.
//!
//! Enable with the `actix` feature.

use crate::de::Config as QsConfig;

#[cfg(feature = "actix-validator")]
use actix_web;
#[cfg(feature = "actix2-validator")]
use actix_web2 as actix_web;
#[cfg(any(feature = "actix-validator", feature = "actix2-validator"))]
use validator::Validate;

use actix_web::dev::Payload;
use actix_web::{Error as ActixError, FromRequest, HttpRequest};
use futures::future::{ready, Ready};
use serde::de;
use std::fmt;
use std::fmt::{Debug, Display};
use std::ops::{Deref, DerefMut};
use crate::actix::QsQueryConfig;
use crate::Error;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
/// Extract typed information from from the request's query.
///
/// ## Example
///
/// ```rust
/// # #[macro_use] extern crate serde_derive;
/// # #[cfg(feature = "actix")]
/// # use actix_web;
/// # #[cfg(feature = "actix2")]
/// # use actix_web2 as actix_web;
/// use actix_web::{web, App, HttpResponse};
/// use serde_qs::actix::QsQuery;
///
/// #[derive(Deserialize)]
/// pub struct UsersFilter {
///    id: Vec<u64>,
/// }
///
/// // Use `QsQuery` extractor for query information.
/// // The correct request for this handler would be `/users?id[]=1124&id[]=88"`
/// fn filter_users(info: QsQuery<UsersFilter>) -> HttpResponse {
///     info.id.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(", ").into()
/// }
///
/// fn main() {
///     let app = App::new().service(
///        web::resource("/users")
///            .route(web::get().to(filter_users)));
/// }
/// ```
pub struct ValidateQsQuery<T>(T);

impl<T> Deref for ValidateQsQuery<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for ValidateQsQuery<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> ValidateQsQuery<T> {
    /// Deconstruct to a inner value
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: Debug> Debug for ValidateQsQuery<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: Display> Display for ValidateQsQuery<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> FromRequest for ValidateQsQuery<T>
    where
        T: de::DeserializeOwned + Validate,
{
    type Error = ActixError;
    type Future = Ready<Result<Self, ActixError>>;
    type Config = QsQueryConfig;

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let query_config = req.app_data::<QsQueryConfig>();

        let error_handler = query_config.map(|c| c.ehandler.clone()).unwrap_or(None);

        let default_qsconfig = QsConfig::default();
        let qsconfig = query_config
            .map(|c| &c.qs_config)
            .unwrap_or(&default_qsconfig);

        let res = qsconfig
            .deserialize_str::<T>(req.query_string())
            .map(|val| ValidateQsQuery(val))
            .and_then(|value| {
                value
                    .validate()
                    .map(move |_| value)
                    .map_err(Error::Validate)
            })
            .map(|val| Ok(val))
            .unwrap_or_else(move |e| {
                let e = if let Some(error_handler) = error_handler {
                    (error_handler)(e, req)
                } else {
                    e.into()
                };

                Err(e)
            });
        ready(res)
    }
}
