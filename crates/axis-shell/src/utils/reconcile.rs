use std::collections::{HashMap, HashSet};

pub fn reconcile<V>(
    map: &mut HashMap<String, V>,
    active_keys: &[impl AsRef<str>],
    on_remove: impl Fn(String, V),
) {
    let active: HashSet<&str> = active_keys.iter().map(|k| k.as_ref()).collect();

    let stale: Vec<String> = map
        .keys()
        .filter(|k| !active.contains(k.as_str()))
        .cloned()
        .collect();

    for key in stale {
        if let Some(value) = map.remove(&key) {
            on_remove(key, value);
        }
    }
}
