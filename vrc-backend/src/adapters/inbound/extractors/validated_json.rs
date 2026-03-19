use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use axum::Json;
use axum::extract::{FromRequest, Request};
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;

use crate::errors::api::ApiError;

pub trait ValidatedPayload {
    fn validate_payload(&self) -> Result<(), HashMap<String, String>>;
    fn validation_error(errors: HashMap<String, String>) -> ApiError;
}

pub struct ValidatedJson<T>(pub T);

impl<T> ValidatedJson<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for ValidatedJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for ValidatedJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + ValidatedPayload + Send,
    Json<T>: FromRequest<S>,
    <Json<T> as FromRequest<S>>::Rejection: IntoResponse,
{
    type Rejection = Response;

    fn from_request(
        req: Request,
        state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let Json(payload) = Json::<T>::from_request(req, state)
                .await
                .map_err(IntoResponse::into_response)?;

            if let Err(errors) = payload.validate_payload() {
                return Err(T::validation_error(errors).into_response());
            }

            Ok(Self(payload))
        }
    }
}