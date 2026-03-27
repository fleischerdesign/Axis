use super::provider::{AuthStatus, Task, TaskList, TaskProvider};
use super::utils::{api_get, api_patch, api_post, build_http_client, config_dir, load_json, save_json};
use log::{info, warn};
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_TASKS_SCOPE: &str = "https://www.googleapis.com/auth/tasks";
const REDIRECT_URI: &str = "http://localhost:8099/callback";

// ── Data Structures ───────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct Credentials {
    client_id: String,
    client_secret: String,
}

#[derive(Serialize, Deserialize, Default)]
struct TokenStore {
    refresh_token: Option<String>,
}

// ── Provider ──────────────────────────────────────────────────────────

pub struct GoogleTasksProvider {
    client_id: String,
    client_secret: String,
    refresh_token: Option<String>,
    access_token: Option<String>,
    config_dir: PathBuf,
    http_client: reqwest::blocking::Client,
}

impl GoogleTasksProvider {
    pub fn load() -> Result<Self, String> {
        let config_dir = config_dir()?;
        let cred_path = config_dir.join("google_credentials.json");
        let cred_json = std::fs::read_to_string(&cred_path)
            .map_err(|_| format!("Missing {cred_path:?}. Create it with client_id + client_secret."))?;
        let creds: Credentials = serde_json::from_str(&cred_json)
            .map_err(|e| format!("Invalid credentials JSON: {e}"))?;

        let token_path = config_dir.join("google_token.json");
        let token_store: TokenStore = load_json(&token_path);

        Ok(Self {
            client_id: creds.client_id,
            client_secret: creds.client_secret,
            refresh_token: token_store.refresh_token,
            access_token: None,
            config_dir,
            http_client: build_http_client(),
        })
    }

    // ── Token Management ───────────────────────────────────────────────

    fn save_token(&self) {
        let token_path = self.config_dir.join("google_token.json");
        let store = TokenStore {
            refresh_token: self.refresh_token.clone(),
        };
        save_json(&token_path, &store);
    }

    fn ensure_access_token(&mut self) -> Result<String, String> {
        if let Some(ref token) = self.access_token {
            return Ok(token.clone());
        }

        let refresh_token = self
            .refresh_token
            .clone()
            .ok_or_else(|| "Not authenticated".to_string())?;

        let client = BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_client_secret(ClientSecret::new(self.client_secret.clone()))
            .set_auth_uri(AuthUrl::new(GOOGLE_AUTH_URL.to_string()).map_err(|e| e.to_string())?)
            .set_token_uri(TokenUrl::new(GOOGLE_TOKEN_URL.to_string()).map_err(|e| e.to_string())?);

        let token_result = client
            .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token))
            .add_scope(Scope::new(GOOGLE_TASKS_SCOPE.to_string()))
            .request(&build_http_client())
            .map_err(|e| format!("Token refresh failed: {e}"))?;

        let access_token = token_result.access_token().secret().clone();
        self.access_token = Some(access_token.clone());

        if let Some(new_refresh) = token_result.refresh_token() {
            self.refresh_token = Some(new_refresh.secret().clone());
            self.save_token();
        }

        Ok(access_token)
    }

    // ── Auth Code Flow (DRY wrapper) ───────────────────────────────────

    fn execute_auth_flow(&mut self) -> Result<(), String> {
        let client = BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_client_secret(ClientSecret::new(self.client_secret.clone()))
            .set_auth_uri(AuthUrl::new(GOOGLE_AUTH_URL.to_string()).map_err(|e| e.to_string())?)
            .set_token_uri(TokenUrl::new(GOOGLE_TOKEN_URL.to_string()).map_err(|e| e.to_string())?)
            .set_redirect_uri(
                RedirectUrl::new(REDIRECT_URI.to_string()).map_err(|e| e.to_string())?,
            );
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, _csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new(GOOGLE_TASKS_SCOPE.to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        info!("[google-tasks] Auth URL: {auth_url}");

        let listener = TcpListener::bind("127.0.0.1:8099")
            .map_err(|e| format!("Cannot bind localhost:8099 — {e}"))?;

        match open::that(auth_url.as_str()) {
            Ok(_) => info!("[google-tasks] Browser opened"),
            Err(e) => warn!("[google-tasks] Failed to open browser: {e}"),
        }

        info!("[google-tasks] Waiting for redirect...");
        let auth_code = wait_for_redirect(&listener)?;

        info!("[google-tasks] Got auth code, exchanging for token...");
        let token_result = client
            .exchange_code(auth_code)
            .set_pkce_verifier(pkce_verifier)
            .request(&build_http_client())
            .map_err(|e| format!("Token exchange failed: {e}"))?;

        let refresh = token_result
            .refresh_token()
            .ok_or("No refresh token received")?
            .secret()
            .clone();

        self.refresh_token = Some(refresh);
        self.access_token = Some(token_result.access_token().secret().clone());
        self.save_token();

        info!("[google-tasks] Auth complete!");
        Ok(())
    }

    // ── HTTP Helpers (DRY via utils) ───────────────────────────────────

    fn api_get<T: serde::de::DeserializeOwned>(&mut self, url: &str) -> Result<T, String> {
        let token = self.ensure_access_token()?;
        api_get(&self.http_client, url, &token)
    }

    fn api_post<T: serde::de::DeserializeOwned>(
        &mut self,
        url: &str,
        body: &impl serde::Serialize,
    ) -> Result<T, String> {
        let token = self.ensure_access_token()?;
        api_post(&self.http_client, url, &token, body)
    }

    fn api_patch<T: serde::de::DeserializeOwned>(
        &mut self,
        url: &str,
        body: &impl serde::Serialize,
    ) -> Result<T, String> {
        let token = self.ensure_access_token()?;
        api_patch(&self.http_client, url, &token, body)
    }

    fn api_delete(&mut self, url: &str) -> Result<(), String> {
        let token = self.ensure_access_token()?;
        super::utils::api_delete(&self.http_client, url, &token)
    }
}

// ── TaskProvider Implementation ───────────────────────────────────────

impl TaskProvider for GoogleTasksProvider {
    fn name(&self) -> &str {
        "Google Tasks"
    }

    fn icon(&self) -> &str {
        "google-symbolic"
    }

    fn is_async(&self) -> bool {
        true
    }

    fn auth_status(&mut self) -> AuthStatus {
        if self.refresh_token.is_some() {
            AuthStatus::Authenticated
        } else {
            AuthStatus::NeedsAuth {
                url: String::new(),
                code: String::new(),
            }
        }
    }

    fn authenticate(&mut self) -> Result<AuthStatus, String> {
        info!("[google-tasks] Starting auth code flow...");
        self.execute_auth_flow()?;
        Ok(AuthStatus::Authenticated)
    }

    fn is_authenticated(&self) -> bool {
        self.refresh_token.is_some()
    }

    fn lists(&mut self) -> Result<Vec<TaskList>, String> {
        #[derive(Deserialize)]
        struct Response {
            items: Option<Vec<Item>>,
        }
        #[derive(Deserialize)]
        struct Item {
            id: String,
            title: String,
        }

        let resp: Response = self
            .api_get("https://tasks.googleapis.com/tasks/v1/users/@me/lists")?;

        Ok(resp
            .items
            .unwrap_or_default()
            .into_iter()
            .map(|i| TaskList {
                id: i.id,
                title: i.title,
            })
            .collect())
    }

    fn tasks(&mut self, list_id: &str) -> Result<Vec<Task>, String> {
        #[derive(Deserialize)]
        struct Response {
            items: Option<Vec<Item>>,
        }
        #[derive(Deserialize)]
        struct Item {
            id: String,
            title: Option<String>,
            status: Option<String>,
        }

        let url = format!(
            "https://tasks.googleapis.com/tasks/v1/lists/{}/tasks?showCompleted=true&maxResults=50",
            list_id
        );

        let resp: Response = self.api_get(&url)?;

        Ok(resp
            .items
            .unwrap_or_default()
            .into_iter()
            .filter_map(|i| {
                let title = i.title?;
                if title.is_empty() {
                    return None;
                }
                Some(Task {
                    id: i.id,
                    title,
                    done: i.status.as_deref() == Some("completed"),
                    provider: "google".to_string(),
                })
            })
            .collect())
    }

    fn add_task(&mut self, list_id: &str, title: &str) -> Result<Task, String> {
        #[derive(Serialize)]
        struct NewTask {
            title: String,
        }
        #[derive(Deserialize)]
        struct CreatedTask {
            id: String,
            title: Option<String>,
        }

        let url = format!(
            "https://tasks.googleapis.com/tasks/v1/lists/{}/tasks",
            list_id
        );

        let created: CreatedTask =
            self.api_post(&url, &NewTask { title: title.to_string() })?;

        info!("[google-tasks] Added: {}", title);

        Ok(Task {
            id: created.id,
            title: created.title.unwrap_or_else(|| title.to_string()),
            done: false,
            provider: "google".to_string(),
        })
    }

    fn toggle_task(&mut self, list_id: &str, task_id: &str, done: bool) -> Result<(), String> {
        #[derive(Serialize)]
        struct PatchBody {
            status: String,
        }

        let url = format!(
            "https://tasks.googleapis.com/tasks/v1/lists/{}/tasks/{}",
            list_id, task_id
        );

        let status = if done { "completed" } else { "needsAction" };

        let _: serde_json::Value = self.api_patch(&url, &PatchBody { status: status.to_string() })?;

        info!("[google-tasks] Toggled {} -> {}", task_id, done);
        Ok(())
    }

    fn delete_task(&mut self, list_id: &str, task_id: &str) -> Result<(), String> {
        let url = format!(
            "https://tasks.googleapis.com/tasks/v1/lists/{}/tasks/{}",
            list_id, task_id
        );

        self.api_delete(&url)?;

        info!("[google-tasks] Deleted: {}", task_id);
        Ok(())
    }
}

// ── OAuth Redirect Handler ────────────────────────────────────────────

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
                let mut error = None;
                for pair in query_str.split('&') {
                    if let Some((key, val)) = pair.split_once('=') {
                        match key {
                            "code" => code = Some(val.to_string()),
                            "error" => error = Some(val.to_string()),
                            _ => {}
                        }
                    }
                }

                if let Some(code) = code {
                    let body = "<html><body style='background:#1e1e1e;color:white;font-family:sans-serif;text-align:center;padding:40px'><h1>Anmeldung erfolgreich!</h1><p>Du kannst dieses Fenster schließen.</p></body></html>";
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes());
                    info!("[google-tasks] Auth code received");
                    return Ok(AuthorizationCode::new(code));
                } else if let Some(error) = error {
                    let body = "<html><body style='background:#1e1e1e;color:#e06c75;font-family:sans-serif;text-align:center;padding:40px'><h1>Anmeldung fehlgeschlagen</h1></body></html>";
                    let response = format!(
                        "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes());
                    return Err(format!("Auth error: {error}"));
                }
            }
        }

        let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
        let _ = stream.write_all(response.as_bytes());
    }

    Err("No redirect received".to_string())
}
