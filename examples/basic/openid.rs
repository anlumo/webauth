use openidconnect::{
    AccessTokenHash, AuthorizationCode, ClientId, CsrfToken, IssuerUrl, Nonce, OAuth2TokenResponse,
    PkceCodeChallenge, RedirectUrl, Scope, TokenResponse,
    core::{
        CoreAuthenticationFlow, CoreClient, CoreIdTokenClaims, CoreProviderMetadata,
        CoreTokenResponse,
    },
};
use url::Url;

use crate::http_client::BasicHttpClient;

pub async fn run(
    issuer_url: String,
    client_id: String,
    redirect_url: Url,
    get_code: impl AsyncFnOnce(Url) -> anyhow::Result<String>,
) -> anyhow::Result<(CoreTokenResponse, CoreIdTokenClaims)> {
    let http_client = BasicHttpClient::new().await?;

    let provider_metadata =
        CoreProviderMetadata::discover_async(IssuerUrl::new(issuer_url)?, &http_client).await?;

    // Create an OpenID Connect client by specifying the client ID, client secret, authorization URL
    // and token URL.
    let client =
        CoreClient::from_provider_metadata(provider_metadata, ClientId::new(client_id), None)
            .set_redirect_uri(RedirectUrl::from_url(redirect_url));

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate the full authorization URL.
    let (auth_url, _csrf_token, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("read".to_string()))
        .add_scope(Scope::new("write".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    let authorization_code = get_code(auth_url).await?;

    // Now you can exchange it for an access token and ID token.
    let token_response = client
        .exchange_code(AuthorizationCode::new(authorization_code))?
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http_client)
        .await?;

    // Extract the ID token claims after verifying its authenticity and nonce.
    let id_token = token_response
        .id_token()
        .ok_or_else(|| anyhow::anyhow!("Server did not return an ID token"))?;
    let id_token_verifier = client.id_token_verifier();
    let claims = id_token.claims(&id_token_verifier, &nonce)?.clone();

    if let Some(expected_access_token_hash) = claims.access_token_hash() {
        let actual_access_token_hash = AccessTokenHash::from_token(
            token_response.access_token(),
            id_token.signing_alg()?,
            id_token.signing_key(&id_token_verifier)?,
        )?;
        if actual_access_token_hash != *expected_access_token_hash {
            anyhow::bail!("Invalid access token");
        }
    }

    Ok((token_response, claims))
}
