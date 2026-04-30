use axis_domain::ports::cloud_auth::{CloudAuthProvider, AuthError};
use axis_domain::models::cloud::{CloudAccount, AccountStatus};
use async_trait::async_trait;
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v3/userinfo";
const REDIRECT_PORT: u16 = 8080;
const REDIRECT_URI: &str = "http://localhost:8080/callback";

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct StoredToken {
    refresh_token: Option<String>,
    access_token: Option<String>,
    expires_at: Option<i64>,
    granted_scopes: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct GoogleCredential {
    client_id: String,
    client_secret: String,
}

#[derive(Deserialize)]
struct GoogleUserInfo {
    sub: String,
    email: String,
}

pub struct GoogleCloudAdapter {
    config_dir: PathBuf,
    token: Arc<Mutex<StoredToken>>,
    credentials: Option<GoogleCredential>,
}

impl GoogleCloudAdapter {
    pub fn new(config_dir: PathBuf) -> Self {
        let cred_path = config_dir.join("google_credentials.json");
        let credentials = std::fs::read_to_string(cred_path)
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok());

        let token_path = config_dir.join("google_token.json");
        let token_data = std::fs::read_to_string(token_path)
            .ok()
            .and_then(|json| serde_json::from_str::<StoredToken>(&json).ok())
            .unwrap_or_default();

        Self {
            config_dir,
            token: Arc::new(Mutex::new(token_data)),
            credentials,
        }
    }

    fn save_token(&self, token: &StoredToken) {
        let token_path = self.config_dir.join("google_token.json");
        if let Ok(json) = serde_json::to_string_pretty(token) {
             let _ = std::fs::write(token_path, json);
        }
    }
}

#[async_trait]
impl CloudAuthProvider for GoogleCloudAdapter {
    async fn authenticate(&self, scopes: &[String]) -> Result<CloudAccount, AuthError> {
        let cred = self.credentials.as_ref().ok_or_else(|| AuthError::ProviderError("Missing google_credentials.json".into()))?;

        let client = BasicClient::new(ClientId::new(cred.client_id.clone()))
            .set_client_secret(ClientSecret::new(cred.client_secret.clone()))
            .set_auth_uri(AuthUrl::new(GOOGLE_AUTH_URL.to_string()).unwrap())
            .set_token_uri(TokenUrl::new(GOOGLE_TOKEN_URL.to_string()).unwrap())
            .set_redirect_uri(RedirectUrl::new(REDIRECT_URI.to_string()).unwrap());

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, _csrf) = client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(scopes.iter().map(|s| Scope::new(s.clone())))
            .set_pkce_challenge(pkce_challenge)
            .url();

        let _ = open::that(auth_url.as_str());

        let listener = TcpListener::bind(format!("127.0.0.1:{}", REDIRECT_PORT)).await
            .map_err(|e| AuthError::ProviderError(format!("Failed to bind port: {}", e)))?;

        let (tx, mut rx) = tokio::sync::mpsc::channel(1);

        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let (reader, mut writer) = stream.split();
                let mut reader = BufReader::new(reader);
                let mut line = String::new();
                if reader.read_line(&mut line).await.is_ok() {
                    if let Some(query) = line.split_whitespace().nth(1) {
                        if let Some(code) = query.split("code=").nth(1).and_then(|s| s.split('&').next()) {
                            let body = "<html><body><h1>Auth Success</h1><p>You can close this window now.</p></body></html>";
                            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}", body.len(), body);
                            let _ = writer.write_all(response.as_bytes()).await;
                            let _ = writer.flush().await;
                            let _ = tx.send(code.to_string()).await;
                        }
                    }
                }
            }
        });

        let code = rx.recv().await.ok_or(AuthError::Cancelled)?;
        let http_client = reqwest::Client::new();
        
        let token_result = client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(&http_client).await
            .map_err(|e| AuthError::ProviderError(format!("Token exchange failed: {}", e)))?;

        let access_token = token_result.access_token().secret().clone();
        
        // Fetch User Info
        let user_info: GoogleUserInfo = http_client.get(GOOGLE_USERINFO_URL)
            .bearer_auth(&access_token)
            .send().await
            .map_err(|e| AuthError::NetworkFailed(e.to_string()))?
            .json().await
            .map_err(|e| AuthError::ProviderError(format!("Failed to parse userinfo: {}", e)))?;

        let mut token = self.token.lock().await;
        token.access_token = Some(access_token);
        token.refresh_token = token_result.refresh_token().map(|t| t.secret().clone());
        token.expires_at = Some(chrono::Utc::now().timestamp() + 3600);
        token.granted_scopes = scopes.to_vec();
        
        self.save_token(&token);
        
        Ok(CloudAccount {
            id: user_info.sub,
            provider_name: "Google".into(),
            display_name: user_info.email,
            status: AccountStatus::Online,
        })
    }

    async fn get_token(&self, _scopes: &[String]) -> Result<String, AuthError> {
        let mut token = self.token.lock().await;
        
        if let Some(expires) = token.expires_at {
            if expires > chrono::Utc::now().timestamp() + 60 {
                if let Some(ref access_token) = token.access_token {
                    return Ok(access_token.clone());
                }
            }
        }

        let cred = self.credentials.as_ref().ok_or_else(|| AuthError::ProviderError("No credentials".into()))?;
        let refresh_token = token.refresh_token.as_ref().ok_or(AuthError::ProviderError("No refresh token".into()))?;

        let http_client = reqwest::Client::new();
        let client = BasicClient::new(ClientId::new(cred.client_id.clone()))
            .set_client_secret(ClientSecret::new(cred.client_secret.clone()))
            .set_auth_uri(AuthUrl::new(GOOGLE_AUTH_URL.to_string()).unwrap())
            .set_token_uri(TokenUrl::new(GOOGLE_TOKEN_URL.to_string()).unwrap());

        let token_result = client
            .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token.clone()))
            .request_async(&http_client).await
            .map_err(|e| AuthError::ProviderError(format!("Token refresh failed: {}", e)))?;

        token.access_token = Some(token_result.access_token().secret().clone());
        token.expires_at = Some(chrono::Utc::now().timestamp() + 3600);
        
        let access = token.access_token.clone()
            .ok_or_else(|| AuthError::ProviderError("No access token after refresh".into()))?;
        self.save_token(&token);

        Ok(access)
    }

    async fn is_authenticated(&self) -> Result<bool, AuthError> {
        Ok(self.token.lock().await.refresh_token.is_some())
    }
}
