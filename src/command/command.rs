pub trait Command: Send + Sync {
    fn description(&self) -> &str;
    fn get_redirect_url(&self, query: &str) -> String;
}
