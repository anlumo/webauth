use std::sync::Arc;

use nyquest::{AsyncClient, ClientBuilder, Method, Request};
use openidconnect::{
    AsyncHttpClient,
    http::{Response, StatusCode, header::CONTENT_TYPE},
};

pub struct BasicHttpClient {
    client: Arc<AsyncClient>,
}

impl BasicHttpClient {
    pub async fn new() -> nyquest::Result<Self> {
        Ok(Self {
            client: Arc::new(
                ClientBuilder::default()
                    .no_redirects()
                    .no_caching()
                    .build_async()
                    .await?,
            ),
        })
    }
}

impl<'c> AsyncHttpClient<'c> for BasicHttpClient {
    type Error = nyquest::Error;
    type Future =
        std::pin::Pin<Box<dyn Future<Output = Result<Response<Vec<u8>>, Self::Error>> + 'c>>;

    fn call(&'c self, request: openidconnect::HttpRequest) -> Self::Future {
        let client = self.client.clone();
        let (header, body) = request.into_parts();
        tracing::trace!("Requesting {:?}", header.uri);
        Box::pin(async move {
            let content_type = header
                .headers
                .get(CONTENT_TYPE)
                .map(|val| val.to_str().unwrap())
                .unwrap_or("application/octet-stream");
            tracing::debug!("Body: {:?}", str::from_utf8(&body));
            let mut ny_request = Request::new(
                Method::custom(header.method.as_str().to_owned()),
                header.uri.to_string(),
            )
            .with_body(nyquest::Body::bytes(body, content_type.to_owned()));

            for (key, value) in header.headers.iter() {
                if key != CONTENT_TYPE {
                    tracing::trace!("Adding header \"{key}: {value:?}\"");
                    ny_request = ny_request
                        .with_header(key.as_str().to_owned(), value.to_str().unwrap().to_owned());
                }
            }

            let ny_response = client.request(ny_request).await?;
            let response_builder = openidconnect::http::response::Builder::new()
                .status(StatusCode::from_u16(ny_response.status().code()).unwrap());
            tracing::trace!("Got response: {ny_response:?}");
            let bytes = ny_response.bytes().await?;

            Ok(response_builder.body(bytes).unwrap())
        })
    }
}
