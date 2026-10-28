// The dashboard page and its files, built into the program.

// The file served at `path` ("/" is the page): its text and content type.
pub fn asset(path: &str) -> Option<(&'static str, &'static str)> {
    match path {
        "/" | "/index.html" => Some((include_str!("index.html"), "text/html; charset=utf-8")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn serves_the_page() {
        assert!(super::asset("/").unwrap().0.contains("<title>Claude Usage</title>"));
        assert!(super::asset("/nope").is_none());
    }
}
