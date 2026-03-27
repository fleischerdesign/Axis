use crate::services::google::{GoogleAuthRegistry, DEFAULT_SCOPES};

pub fn authenticate_async<F>(on_complete: F)
where
    F: FnOnce(Result<(), String>) + Send + 'static,
{
    let scopes = DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    
    std::thread::spawn(move || {
        let scopes_ref: Vec<&str> = scopes.iter().map(|s| s.as_str()).collect();
        
        let mut registry = match GoogleAuthRegistry::load() {
            Ok(r) => r,
            Err(e) => {
                on_complete(Err(e));
                return;
            }
        };

        match registry.execute_auth_flow(&scopes_ref) {
            Ok(()) => {
                log::info!("[google-auth] Auth successful");
                on_complete(Ok(()));
            }
            Err(e) => {
                log::warn!("[google-auth] Auth failed: {}", e);
                on_complete(Err(e));
            }
        }
    });
}