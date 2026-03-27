use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::PathBuf;

// ── XDG Directory Resolution ──────────────────────────────────────────

pub fn config_dir() -> Result<PathBuf, String> {
    xdg_dir("XDG_CONFIG_HOME", ".config/axis")
}

pub fn data_dir() -> Option<PathBuf> {
    xdg_dir("XDG_DATA_HOME", ".local/share/axis").ok()
}

fn xdg_dir(xdg_var: &str, fallback_suffix: &str) -> Result<PathBuf, String> {
    std::env::var_os(xdg_var)
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| {
                PathBuf::from(h).join(fallback_suffix)
            })
        })
        .ok_or_else(|| format!("Cannot determine directory ({xdg_var} or HOME not set)"))
}

// ── JSON File I/O ─────────────────────────────────────────────────────

pub fn load_json<T: DeserializeOwned + Default>(path: &PathBuf) -> T {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_json<T: Serialize>(path: &PathBuf, value: &T) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, json);
    }
}

// ── HTTP Helpers ───────────────────────────────────────────────────────

pub fn build_http_client() -> reqwest::blocking::Client {
    reqwest::blocking::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("HTTP client should build")
}

pub fn api_get<T: DeserializeOwned>(
    client: &reqwest::blocking::Client,
    url: &str,
    token: &str,
) -> Result<T, String> {
    api_request(client, reqwest::Method::GET, url, token, None::<&()>)
}

pub fn api_post<T: DeserializeOwned>(
    client: &reqwest::blocking::Client,
    url: &str,
    token: &str,
    body: &impl Serialize,
) -> Result<T, String> {
    api_request(client, reqwest::Method::POST, url, token, Some(body))
}

pub fn api_patch<T: DeserializeOwned>(
    client: &reqwest::blocking::Client,
    url: &str,
    token: &str,
    body: &impl Serialize,
) -> Result<T, String> {
    api_request(client, reqwest::Method::PATCH, url, token, Some(body))
}

fn api_request<T: DeserializeOwned, B: Serialize>(
    client: &reqwest::blocking::Client,
    method: reqwest::Method,
    url: &str,
    token: &str,
    body: Option<&B>,
) -> Result<T, String> {
    let mut req = client
        .request(method, url)
        .bearer_auth(token);

    if let Some(b) = body {
        req = req.json(b);
    }

    let resp = req.send().map_err(|e| format!("API request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        return Err(format!("API error {status}: {text}"));
    }

    resp.json::<T>().map_err(|e| format!("JSON parse failed: {e}"))
}
