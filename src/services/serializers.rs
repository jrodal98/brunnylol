// Serializers for bookmark import/export

use anyhow::Result;
use crate::config::yml_settings::YmlSettings;

/// Strategy trait for import/export formats
pub trait BookmarkSerializer: Send + Sync {
    fn serialize(&self, bookmarks: &[YmlSettings]) -> Result<String>;
    fn deserialize(&self, content: &str) -> Result<Vec<YmlSettings>>;
    fn file_extension(&self) -> &'static str;
    fn content_type(&self) -> &'static str;
}

/// YAML serializer (existing format)
pub struct YamlSerializer;

impl BookmarkSerializer for YamlSerializer {
    fn serialize(&self, bookmarks: &[YmlSettings]) -> Result<String> {
        Ok(serde_yaml::to_string(bookmarks)?)
    }

    fn deserialize(&self, content: &str) -> Result<Vec<YmlSettings>> {
        Ok(serde_yaml::from_str(content)?)
    }

    fn file_extension(&self) -> &'static str {
        "yml"
    }

    fn content_type(&self) -> &'static str {
        "application/x-yaml"
    }
}

/// JSON serializer (new format)
pub struct JsonSerializer;

impl BookmarkSerializer for JsonSerializer {
    fn serialize(&self, bookmarks: &[YmlSettings]) -> Result<String> {
        Ok(serde_json::to_string_pretty(bookmarks)?)
    }

    fn deserialize(&self, content: &str) -> Result<Vec<YmlSettings>> {
        Ok(serde_json::from_str(content)?)
    }

    fn file_extension(&self) -> &'static str {
        "json"
    }

    fn content_type(&self) -> &'static str {
        "application/json"
    }
}
