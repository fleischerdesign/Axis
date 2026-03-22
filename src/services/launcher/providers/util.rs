/// Case-insensitive text matching score.
/// exact match = 100, starts_with = 80, contains = 50, no match = 0.
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

/// Score a title + optional description against a query.
/// Title match is preferred; if title doesn't match, tries description.
pub fn scored_match(title: &str, description: Option<&str>, query: &str) -> i32 {
    let s = text_score(title, query);
    if s > 0 {
        return s;
    }
    // Description match only returns the actual score — no free points.
    // Matches in description score 50 (contains) or 80 (starts_with),
    // matching the old behaviour where description hits scored 30-50.
    description.map_or(0, |d| text_score(d, query))
}
