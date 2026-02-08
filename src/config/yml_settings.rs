use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct YmlSettings {
    pub alias: String,
    pub description: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encode: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
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

    #[test]
    fn test_skip_none_fields_in_yaml() {
        let settings = YmlSettings {
            alias: "g".to_string(),
            description: "Google Search".to_string(),
            url: "https://google.com".to_string(),
            command: None,
            encode: None,
            nested: None,
        };

        let yml = serde_yaml::to_string(&settings).unwrap();

        // Verify that None fields are not serialized
        assert!(!yml.contains("command:"));
        assert!(!yml.contains("encode:"));
        assert!(!yml.contains("nested:"));

        // Verify that required fields are present
        assert!(yml.contains("alias:"));
        assert!(yml.contains("description:"));
        assert!(yml.contains("url:"));
    }

    #[test]
    fn test_skip_none_fields_in_json() {
        let settings = YmlSettings {
            alias: "g".to_string(),
            description: "Google Search".to_string(),
            url: "https://google.com".to_string(),
            command: None,
            encode: None,
            nested: None,
        };

        let json = serde_json::to_string(&settings).unwrap();

        // Verify that None fields are not serialized
        assert!(!json.contains("\"command\""));
        assert!(!json.contains("\"encode\""));
        assert!(!json.contains("\"nested\""));

        // Verify that required fields are present
        assert!(json.contains("\"alias\""));
        assert!(json.contains("\"description\""));
        assert!(json.contains("\"url\""));
    }

    #[test]
    fn test_include_some_fields() {
        let settings = YmlSettings {
            alias: "gh".to_string(),
            description: "GitHub".to_string(),
            url: "https://github.com".to_string(),
            command: Some("{user}/{repo}".to_string()),
            encode: Some(true),
            nested: None,
        };

        let json = serde_json::to_string(&settings).unwrap();

        // Verify that Some fields are included
        assert!(json.contains("\"command\""));
        assert!(json.contains("\"encode\""));

        // Verify that None field is not included
        assert!(!json.contains("\"nested\""));
    }
}
