use super::Command;

/// A struct that represents a command that navigates to a pre-defined URL when executed.
pub struct BookmarkCommand {
    bookmark: String,
    description: String,
}

impl Command for BookmarkCommand {
    fn description(&self) -> String {
        self.description.clone()
    }

    fn get_redirect_url(&self, _query: &str) -> String {
        self.bookmark.clone()
    }
}

impl BookmarkCommand {
    pub fn new(bookmark: &str, description: &str) -> Self {
        Self {
            bookmark: bookmark.to_string(),
            description: description.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_description() {
        let bookmark = BookmarkCommand::new("www.example.com", "a test website");
        assert_eq!(bookmark.description(), "a test website".to_string());
    }

    #[test]
    fn test_empty_query_redirect() {
        let bookmark = BookmarkCommand::new("www.example.com", "a test website");
        assert_eq!(bookmark.get_redirect_url(""), "www.example.com".to_string());
    }

    #[test]
    fn test_non_empty_query_redirect() {
        let bookmark = BookmarkCommand::new("www.example.com", "a test website");
        assert_eq!(
            bookmark.get_redirect_url("hello world"),
            "www.example.com".to_string()
        );
    }
}
