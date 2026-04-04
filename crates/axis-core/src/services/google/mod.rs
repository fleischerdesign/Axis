pub mod token;

use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;

pub const DEFAULT_SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/userinfo.profile",
    "https://www.googleapis.com/auth/userinfo.email",
];

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REDIRECT_PORT: u16 = 8080;
const REDIRECT_URI: &str = "http://localhost:8080/callback";

// ── Shared Auth Helpers ────────────────────────────────────────────────
//
// Free functions used by all Google-based providers (Tasks, Calendar).
// Each provider just calls these instead of duplicating the logic.

use crate::services::tasks::provider::AuthStatus;

pub fn google_auth_status() -> AuthStatus {
    match GoogleAuthRegistry::load() {
        Ok(reg) if reg.is_authenticated() => AuthStatus::Authenticated,
        _ => AuthStatus::NeedsAuth { url: String::new(), code: None },
    }
}

pub fn google_authenticate(scopes: &[String]) {
    let scopes_owned: Vec<String> = scopes.to_vec();
    GoogleAuthRegistry::authenticate(&scopes_owned, |result| {
        match result {
            Ok(()) => log::info!("[google-auth] Auth successful"),
            Err(e) => log::warn!("[google-auth] Auth failed: {e}"),
        }
    });
}

pub fn google_is_authenticated() -> bool {
    GoogleAuthRegistry::load().map(|r| r.is_authenticated()).unwrap_or(false)
}

#[derive(Serialize, Deserialize)]
pub struct GoogleCredential {
    pub client_id: String,
    pub client_secret: String,
}

impl GoogleCredential {
    pub fn load(config_dir: &PathBuf) -> Result<Self, String> {
        let cred_path = config_dir.join("google_credentials.json");
        let cred_json = std::fs::read_to_string(&cred_path)
            .map_err(|_| format!("Missing {cred_path:?}. Create it with client_id + client_secret."))?;
        let creds: GoogleCredential = serde_json::from_str(&cred_json)
            .map_err(|e| format!("Invalid credentials JSON: {e}"))?;
        Ok(creds)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct StoredToken {
    pub refresh_token: Option<String>,
    pub access_token: Option<String>,
    pub expires_at: Option<i64>,
    pub granted_scopes: Vec<String>,
}

pub struct GoogleAuthRegistry {
    credential: Option<GoogleCredential>,
    config_dir: PathBuf,
    http_client: reqwest::blocking::Client,
    token: StoredToken,
}

impl GoogleAuthRegistry {
    pub fn load() -> Result<Self, String> {
        let config_dir = crate::services::tasks::utils::config_dir()?;
        
        let credential = GoogleCredential::load(&config_dir).ok();
        let token_path = config_dir.join("google_token.json");
        let token: StoredToken = crate::services::tasks::utils::load_json(&token_path);

        Ok(Self {
            credential,
            config_dir,
            http_client: crate::services::tasks::utils::build_http_client()?,
            token,
        })
    }

    pub fn is_authenticated(&self) -> bool {
        self.token.refresh_token.is_some()
    }

    pub fn has_all_scopes(&self, required: &[&str]) -> bool {
        let required: Vec<String> = required.iter().map(|s| s.to_string()).collect();
        required.iter().all(|r| self.token.granted_scopes.contains(r))
    }

    pub fn ensure_token(&mut self, required_scopes: &[&str]) -> Result<String, String> {
        let cred = self.credential.as_ref().ok_or("No credentials")?;

        if !self.is_authenticated() {
            return Err("Not authenticated".to_string());
        }

        if !self.has_all_scopes(required_scopes) {
            let refresh = self.token.refresh_token.as_ref().map(|s| s.as_str());
            let new_token = token::refresh_token(
                &cred.client_id,
                &cred.client_secret,
                refresh,
                required_scopes,
            )?;
            self.token.access_token = Some(new_token.clone());
            self.token.expires_at = Some(chrono::Utc::now().timestamp() + 3600);
            self.token.granted_scopes = required_scopes.iter().map(|s| s.to_string()).collect();
            self.save_token();
            return Ok(new_token);
        }

        if let Some(expires) = self.token.expires_at {
            if expires > chrono::Utc::now().timestamp() + 60 {
                return self.token.access_token.clone()
                    .ok_or_else(|| "No access token".to_string());
            }
        }

        let new_token = token::refresh_token(
            &cred.client_id,
            &cred.client_secret,
            self.token.refresh_token.as_ref().map(|s| s.as_str()),
            required_scopes,
        )?;

        self.token.access_token = Some(new_token.clone());
        self.token.expires_at = Some(chrono::Utc::now().timestamp() + 3600);
        self.save_token();

        Ok(new_token)
    }

    fn save_token(&self) {
        let token_path = self.config_dir.join("google_token.json");
        crate::services::tasks::utils::save_json(&token_path, &self.token);
    }

    pub fn execute_auth_flow(&mut self, scopes: &[String]) -> Result<(), String> {
        let cred = self.credential.as_ref().ok_or("No credentials")?;

        let client = BasicClient::new(ClientId::new(cred.client_id.clone()))
            .set_client_secret(ClientSecret::new(cred.client_secret.clone()))
            .set_auth_uri(AuthUrl::new(GOOGLE_AUTH_URL.to_string()).map_err(|e| e.to_string())?)
            .set_token_uri(TokenUrl::new(GOOGLE_TOKEN_URL.to_string()).map_err(|e| e.to_string())?)
            .set_redirect_uri(RedirectUrl::new(REDIRECT_URI.to_string()).map_err(|e| e.to_string())?);

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, _csrf) = client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(scopes.iter().map(|s| Scope::new(s.clone())))
            .set_pkce_challenge(pkce_challenge)
            .url();

        log::info!("[google-auth] Auth URL: {}", auth_url.as_str());

        let listener = TcpListener::bind(format!("127.0.0.1:{REDIRECT_PORT}"))
            .map_err(|e| format!("Cannot bind localhost:8099 — {e}"))?;

        match open::that(auth_url.as_str()) {
            Ok(_) => log::info!("[google-auth] Browser opened"),
            Err(e) => log::warn!("[google-auth] Failed to open browser: {e}"),
        }

        let auth_code = wait_for_redirect(&listener)?;

        let token_result = client
            .exchange_code(auth_code)
            .set_pkce_verifier(pkce_verifier)
            .request(&self.http_client)
            .map_err(|e| format!("Token exchange failed: {}", e))?;

        let access_token = token_result.access_token().secret().clone();
        let refresh_token = token_result
            .refresh_token()
            .ok_or("No refresh token received")?
            .secret()
            .clone();

        self.token.refresh_token = Some(refresh_token.clone());
        self.token.access_token = Some(access_token.clone());
        self.token.granted_scopes = scopes.iter().map(|s| s.to_string()).collect();
        self.token.expires_at = Some(chrono::Utc::now().timestamp() + 3600);
        self.save_token();

        Ok(())
    }

    pub fn authenticate<F>(scopes: &[String], on_complete: F)
    where F: FnOnce(Result<(), String>) + Send + 'static
    {
        let scopes_owned: Vec<String> = scopes.to_vec();
        std::thread::spawn(move || {
            let mut registry = match GoogleAuthRegistry::load() {
                Ok(r) => r,
                Err(e) => {
                    on_complete(Err(e));
                    return;
                }
            };

            match registry.execute_auth_flow(&scopes_owned) {
                Ok(()) => on_complete(Ok(())),
                Err(e) => on_complete(Err(e)),
            }
        });
    }
}

fn wait_for_redirect(listener: &TcpListener) -> Result<AuthorizationCode, String> {
    for stream in listener.incoming() {
        let mut stream = stream.map_err(|e| format!("Connection error: {e}"))?;
        let reader = BufReader::new(&stream);
        let request_line = reader
            .lines()
            .next()
            .ok_or("Empty request")?
            .map_err(|e| format!("Read error: {e}"))?;

        if let Some(query) = request_line.split_whitespace().nth(1) {
            if let Some(query_str) = query.strip_prefix("/callback?") {
                let mut code = None;
                for pair in query_str.split('&') {
                    if let Some((key, val)) = pair.split_once('=') {
                        if key == "code" {
                            code = Some(val.to_string());
                        }
                    }
                }

                if let Some(code) = code {
                    let body = r#"<html><body style='background:#1e1e1e;color:white;font-family:sans-serif;text-align:center;padding:40px'><h1>Anmeldung erfolgreich!</h1><p>Du kannst dieses Fenster schließen.</p></body></html>"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes());
                    return Ok(AuthorizationCode::new(code));
                }
            }
        }

        let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
        let _ = stream.write_all(response.as_bytes());
    }

    Err("No redirect received".to_string())
}