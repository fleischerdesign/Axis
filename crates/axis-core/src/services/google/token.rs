use serde::Deserialize;

pub fn refresh_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: Option<&str>,
    scopes: &[&str],
) -> Result<String, String> {
    let refresh_token = refresh_token.ok_or("No refresh token available")?;

    let client = reqwest::blocking::Client::new();

    let scope_str = scopes.join(" ");
    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
        ("scope", &scope_str),
    ];

    let response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .map_err(|e| format!("Token refresh failed: {}", e))?;

    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: String,
    }

    let token: TokenResponse = response
        .json()
        .map_err(|e| format!("Token parse failed: {}", e))?;

    Ok(token.access_token)
}