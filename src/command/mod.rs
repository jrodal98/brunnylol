pub mod bookmark_command;
pub mod nested_command;
pub mod templated_command;

pub trait Command: Send + Sync {
    fn description(&self) -> String;
    fn get_redirect_url(&self, query: &str) -> String;
}
