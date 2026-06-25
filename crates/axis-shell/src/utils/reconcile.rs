use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;

pub fn reconcile<K: Eq + Hash + Clone, V>(
    map: &mut HashMap<K, V>,
    active_keys: &HashSet<K>,
    mut on_remove: impl FnMut(&K, V),
) {
    let stale: Vec<K> = map
        .keys()
        .filter(|k| !active_keys.contains(k))
        .cloned()
        .collect();
    for key in stale {
        if let Some(value) = map.remove(&key) {
            on_remove(&key, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn hs(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn empty_map_nothing_removed() {
        let mut map: HashMap<String, i32> = HashMap::new();
        let removed = Rc::new(RefCell::new(Vec::new()));
        let r = removed.clone();
        reconcile(&mut map, &hs(&[]), move |k, _v| {
            r.borrow_mut().push(k.clone());
        });
        assert!(map.is_empty());
        assert!(removed.borrow().is_empty());
    }

    #[test]
    fn all_active_nothing_removed() {
        let mut map = HashMap::from([("a".into(), 1), ("b".into(), 2)]);
        let removed = Rc::new(RefCell::new(Vec::new()));
        let r = removed.clone();
        reconcile(&mut map, &hs(&["a", "b"]), move |k, _v| {
            r.borrow_mut().push(k.clone());
        });
        assert_eq!(map.len(), 2);
        assert!(removed.borrow().is_empty());
    }

    #[test]
    fn stale_keys_removed_and_callback_fired() {
        let mut map = HashMap::from([("a".into(), 1), ("b".into(), 2), ("c".into(), 3)]);
        let removed = Rc::new(RefCell::new(Vec::new()));
        let r = removed.clone();
        reconcile(&mut map, &hs(&["a", "c"]), move |k, _v| {
            r.borrow_mut().push(k.clone());
        });
        assert_eq!(map.len(), 2);
        assert!(map.contains_key("a"));
        assert!(!map.contains_key("b"));
        assert!(map.contains_key("c"));
        assert_eq!(removed.borrow().len(), 1);
        assert!(removed.borrow().contains(&"b".to_string()));
    }

    #[test]
    fn all_stale_all_removed() {
        let mut map = HashMap::from([("x".into(), 10), ("y".into(), 20)]);
        let removed = Rc::new(RefCell::new(Vec::new()));
        let r = removed.clone();
        reconcile(&mut map, &hs(&[]), move |k, _v| {
            r.borrow_mut().push(k.clone());
        });
        assert!(map.is_empty());
        assert_eq!(removed.borrow().len(), 2);
    }

    #[test]
    fn values_passed_to_callback() {
        let mut map = HashMap::from([("keep".into(), 42), ("drop".into(), 99)]);
        let dropped = Rc::new(RefCell::new(Vec::new()));
        let d = dropped.clone();
        reconcile(&mut map, &hs(&["keep"]), move |_k, v| {
            d.borrow_mut().push(v);
        });
        assert_eq!(map.len(), 1);
        assert_eq!(dropped.borrow().as_slice(), &[99]);
    }
}
