pub fn text_score(text: &str, query: &str) -> i32 {
    let t = text.to_lowercase();
    let q = query.to_lowercase();
    if t == q {
        100
    } else if t.starts_with(&q) {
        80
    } else if t.contains(&q) {
        50
    } else {
        0
    }
}

pub fn scored_match(title: &str, description: Option<&str>, query: &str) -> i32 {
    let s = text_score(title, query);
    if s > 0 {
        return s;
    }
    description.map_or(0, |d| text_score(d, query))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_score_exact_match() {
        assert_eq!(text_score("Firefox", "firefox"), 100);
        assert_eq!(text_score("firefox", "Firefox"), 100);
    }

    #[test]
    fn text_score_prefix_match() {
        assert_eq!(text_score("Firefox", "fire"), 80);
        assert_eq!(text_score("Terminal", "term"), 80);
    }

    #[test]
    fn text_score_contains_match() {
        assert_eq!(text_score("Firefox", "fox"), 50);
        assert_eq!(text_score("Calculator", "lato"), 50);
    }

    #[test]
    fn text_score_no_match() {
        assert_eq!(text_score("Firefox", "chrome"), 0);
        assert_eq!(text_score("", "firefox"), 0);
    }

    #[test]
    fn scored_match_title_over_description() {
        let s = scored_match("Firefox", Some("Web Browser"), "firefox");
        assert_eq!(s, 100);
    }

    #[test]
    fn scored_match_falls_back_to_description() {
        let s = scored_match("", Some("Web Browser"), "browser");
        assert_eq!(s, 50);
    }

    #[test]
    fn scored_match_no_description_fallback() {
        let s = scored_match("Xterm", None, "firefox");
        assert_eq!(s, 0);
    }
}
