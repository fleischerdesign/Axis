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
