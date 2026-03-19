use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use axum::extract::{FromRequestParts, Query};
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;

use crate::errors::api::ApiError;

pub struct ValidatedQuery<T>(pub T);

impl<T> ValidatedQuery<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for ValidatedQuery<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for ValidatedQuery<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S, T> FromRequestParts<S> for ValidatedQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
    Query<T>: FromRequestParts<S>,
{
    type Rejection = Response;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let Query(query) = Query::<T>::from_request_parts(parts, state)
                .await
                .map_err(|_rejection| {
                    ApiError::ValidationError(HashMap::from([(
                        "query".to_owned(),
                        "クエリパラメータが不正です".to_owned(),
                    )]))
                    .into_response()
                })?;

            Ok(Self(query))
        }
    }
}