use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct YmlSettings {
    pub alias: String,
    pub description: String,
    pub url: String,
    pub command: Option<String>,
    pub encode: Option<bool>,
    pub nested: Option<Vec<YmlSettings>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde() {
        let settings = YmlSettings {
            alias: "g".to_string(),
            description: "test".to_string(),
            url: "hi".to_string(),
            command: None,
            encode: None,
            nested: None,
        };

        let yml = serde_yaml::to_string(&settings).unwrap();
        let deserde: YmlSettings = serde_yaml::from_str(&yml).unwrap();
        assert_eq!(settings, deserde);
    }
}
