use std::marker::PhantomData;

use derive_more::{Deref, From};
use isahc::{
    http::request::Builder, AsyncBody, AsyncReadResponseExt, Request, RequestExt, Response,
};
use meilisearch_sdk::{Error, RequestClient};
use url::Url;

use self::body_transform::BodyTransform;

pub struct IsahcRequestClient<T: BodyTransform<B>, B>(Builder, PhantomData<T>, PhantomData<B>);

#[async_trait::async_trait]
impl<'a, B: 'a + Send, T: BodyTransform<B>> RequestClient<'a, B> for IsahcRequestClient<T, B> {
    type Request = Result<Request<AsyncBody>, isahc::http::Error>;
    type Response = Response<AsyncBody>;
    type HttpError = IsachError;

    fn new(url: Url) -> Self {
        Self(Builder::new().uri(url.as_str()), PhantomData, PhantomData)
    }

    fn append_header(mut self, name: http::HeaderName, value: http::HeaderValue) -> Self {
        self.0 = self.0.header(name, value);
        self
    }

    fn with_method(mut self, http_method: http::Method) -> Self {
        self.0 = self.0.method(http_method);
        self
    }

    fn add_body(self, body: Option<B>) -> Self::Request {
        match body {
            Some(body) => self.0.body(T::body_transform(body)),
            None => self.0.body(AsyncBody::empty()),
        }
    }

    async fn send_request(request: Self::Request) -> Result<Self::Response, Error> {
        request
            .map_err(|_| Error::InvalidRequest)?
            .send_async()
            .await
            .map_err(|err| IsachError::from(err).into())
    }

    fn extract_status_code(response: &Self::Response) -> u16 {
        response.status().as_u16()
    }

    async fn response_to_text(mut response: Self::Response) -> Result<String, Error> {
        response
            .text()
            .await
            .map_err(|e| IsachError::from(isahc::Error::from(e)).into())
    }
}

mod body_transform {
    use isahc::AsyncBody;
    use serde::Serialize;

    pub trait BodyTransform<B> {
        fn body_transform(body: B) -> AsyncBody;
    }

    pub struct SerializeBodyTransform;
    impl<B: Serialize> BodyTransform<B> for SerializeBodyTransform {
        fn body_transform(body: B) -> AsyncBody {
            AsyncBody::from_bytes_static(
                serde_json::to_string(&body).expect("unable to serialize body"),
            )
        }
    }

    pub struct ReadBodyTransform;
    impl<B: futures_io::AsyncRead + Send + Sync + 'static> BodyTransform<B> for ReadBodyTransform {
        fn body_transform(body: B) -> AsyncBody {
            AsyncBody::from_reader(body)
        }
    }
}

#[derive(Deref, From)]
pub struct IsachError(isahc::Error);
impl From<IsachError> for Error {
    fn from(error: IsachError) -> Error {
        if error.kind() == isahc::error::ErrorKind::ConnectionFailed {
            Error::UnreachableServer
        } else {
            Error::Http(Box::new(error.0))
        }
    }
}
