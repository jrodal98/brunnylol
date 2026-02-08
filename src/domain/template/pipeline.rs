// Pipeline operation trait and implementations

use anyhow::Result;
use std::collections::HashMap;

/// Trait for variable transformations
pub trait PipelineOperation: Send + Sync {
    fn name(&self) -> &'static str;
    fn apply(&self, value: &str) -> Result<String>;
}

/// URL-encode operation
pub struct EncodeOp;

impl PipelineOperation for EncodeOp {
    fn name(&self) -> &'static str {
        "encode"
    }

    fn apply(&self, value: &str) -> Result<String> {
        Ok(urlencoding::encode(value).to_string())
    }
}

/// Trim whitespace operation
pub struct TrimOp;

impl PipelineOperation for TrimOp {
    fn name(&self) -> &'static str {
        "trim"
    }

    fn apply(&self, value: &str) -> Result<String> {
        Ok(value.trim().to_string())
    }
}

/// Registry for pipeline operations
pub struct PipelineRegistry {
    operations: HashMap<&'static str, Box<dyn PipelineOperation>>,
}

impl PipelineRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            operations: HashMap::new(),
        };
        registry.register(Box::new(EncodeOp));
        registry.register(Box::new(TrimOp));
        registry
    }

    pub fn register(&mut self, op: Box<dyn PipelineOperation>) {
        self.operations.insert(op.name(), op);
    }

    pub fn get(&self, name: &str) -> Option<&dyn PipelineOperation> {
        self.operations.get(name).map(|b| b.as_ref())
    }
}

impl Default for PipelineRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_op() {
        let op = EncodeOp;
        assert_eq!(op.apply("hello world").unwrap(), "hello%20world");
        assert_eq!(op.apply("foo/bar").unwrap(), "foo%2Fbar");
    }

    #[test]
    fn test_trim_op() {
        let op = TrimOp;
        assert_eq!(op.apply("  hello  ").unwrap(), "hello");
        assert_eq!(op.apply("world").unwrap(), "world");
    }

    #[test]
    fn test_registry() {
        let registry = PipelineRegistry::new();
        assert!(registry.get("encode").is_some());
        assert!(registry.get("trim").is_some());
        assert!(registry.get("unknown").is_none());
    }
}
