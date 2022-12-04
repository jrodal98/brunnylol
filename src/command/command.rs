pub trait Command: Send + Sync {
    fn description(&self) -> String;
    fn get_redirect_url(&self, query: &str) -> String;
}
