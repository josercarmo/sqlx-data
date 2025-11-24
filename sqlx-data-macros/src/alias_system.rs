use std::collections::HashMap;
use syn::{Attribute, LitStr, Result as SynResult};

/// Helper to create consistent syn::Error instances
fn alias_error(msg: &str) -> syn::Error {
    syn::Error::new(proc_macro2::Span::call_site(), msg)
}

/// Manages SQL aliases for a repository trait
#[derive(Debug, Clone)]
pub struct AliasManager {
    aliases: HashMap<String, String>,
}

impl AliasManager {
    pub fn new() -> Self {
        Self {
            aliases: HashMap::new(),
        }
    }

    /// Parse alias attributes from trait attributes
    pub fn parse_from_attributes(attrs: &[Attribute]) -> SynResult<Self> {
        let mut manager = Self::new();

        for attr in attrs {
            if attr.path().is_ident("alias") {
                manager.parse_alias_attribute(attr)?;
            }
        }

        Ok(manager)
    }

    /// Parse a single alias attribute: #[sqlx_data::alias(name = "content")]
    fn parse_alias_attribute(&mut self, attr: &Attribute) -> SynResult<()> {
        attr.parse_nested_meta(|meta| {
            let ident = meta
                .path
                .get_ident()
                .ok_or_else(|| meta.error("Expected alias name"))?;
            let alias_name = ident.to_string();

            // Expect: alias = "content"
            if meta.input.parse::<syn::Token![=]>().is_err() {
                return Err(meta.error("Expected alias = \"content\" format"));
            }

            let content: LitStr = meta.input.parse()?;
            self.add_alias(alias_name, content.value());

            Ok(())
        })
    }

    /// Add an alias to the manager
    pub fn add_alias(&mut self, name: String, content: String) {
        self.aliases.insert(name, content);
    }

    /// Substitute all aliases in a SQL string
    pub fn substitute_aliases(&self, sql: &str) -> SynResult<String> {
        let mut result = sql.to_string();
        let mut substituted = std::collections::HashSet::new();

        loop {
            let mut changed = false;

            for (alias_name, alias_content) in &self.aliases {
                let pattern = format!("{{{{{}}}}}", alias_name);

                if !result.contains(&pattern) {
                    continue;
                }

                // Check for circular references
                if !substituted.insert(alias_name) {
                    return Err(alias_error(&format!(
                        "Circular alias reference detected: {}",
                        alias_name
                    )));
                }

                result = result.replace(&pattern, alias_content);
                changed = true;
            }

            if !changed {
                break;
            }
        }

        // Validate that all aliases were resolved
        self.validate_no_unresolved_aliases(&result)?;

        Ok(result)
    }

    /// Validate that no unresolved alias patterns remain
    fn validate_no_unresolved_aliases(&self, sql: &str) -> SynResult<()> {
        use crate::constants::regex::ALIAS_PATTERN;

        let Ok(Some(captures)) = ALIAS_PATTERN.captures(sql) else {
            return Ok(());
        };

        let unresolved_alias = &captures[1];
        Err(alias_error(&format!(
            "Unresolved alias: {{{{ {} }}}}",
            unresolved_alias
        )))
    }

    /// Check if the manager has any aliases
    pub fn has_aliases(&self) -> bool {
        !self.aliases.is_empty()
    }

    /// Get all alias names for debugging
    #[allow(dead_code)]
    pub fn get_alias_names(&self) -> Vec<String> {
        self.aliases.keys().cloned().collect()
    }

    /// Serialize aliases to inject as hidden attribute on methods
    pub fn serialize_for_injection(&self) -> String {
        if self.aliases.is_empty() {
            return String::new();
        }

        // Format: "alias1=content1;alias2=content2"
        self.aliases
            .iter()
            .map(|(name, content)| {
                // Escape semicolons and quotes for safe attribute injection
                let escaped_content = content.replace("\"", "\\\"").replace(";", "\\;");
                format!("{}={}", name, escaped_content)
            })
            .collect::<Vec<_>>()
            .join(";")
    }

    /// Deserialize aliases from injected attribute
    pub fn deserialize_from_injection(serialized: &str) -> syn::Result<Self> {
        let mut manager = Self::new();

        if serialized.trim().is_empty() {
            return Ok(manager);
        }

        for pair in serialized.split(';') {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }

            let (name, escaped_content) = pair.split_once('=').ok_or_else(|| {
                alias_error(&format!("Invalid alias format in injection: '{}'", pair))
            })?;

            let content = escaped_content.replace("\\\"", "\"").replace("\\;", ";");
            manager.add_alias(name.trim().to_string(), content);
        }

        Ok(manager)
    }

    /// Extract aliases from method's hidden attribute (used by DML parsing)
    pub fn extract_from_method_attributes(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        for attr in attrs {
            if !attr.path().is_ident("sqlx_data_aliases") {
                continue;
            }

            let meta = match &attr.meta {
                syn::Meta::NameValue(meta) => meta,
                _ => continue, // Ignore unexpected meta formats
            };

            let lit_str = match &meta.value {
                syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
                    syn::Lit::Str(lit_str) => lit_str,
                    _ => continue, // Ignore non-string literals
                },
                _ => continue, // Ignore non-literal expressions
            };

            return Self::deserialize_from_injection(&lit_str.value());
        }
        // No aliases found
        Ok(Self::new())
    }
}

impl Default for AliasManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_simple_alias_substitution() {
        let mut manager = AliasManager::new();
        manager.add_alias("all".to_string(), "id, name, email".to_string());

        let sql = "SELECT {{all}} FROM users";
        let result = manager.substitute_aliases(sql).unwrap();
        assert_eq!(result, "SELECT id, name, email FROM users");
    }

    #[test]
    fn test_multiple_alias_substitution() {
        let mut manager = AliasManager::new();
        manager.add_alias("columns".to_string(), "id, name".to_string());
        manager.add_alias("table".to_string(), "users u".to_string());

        let sql = "SELECT {{columns}} FROM {{table}} WHERE u.active = 1";
        let result = manager.substitute_aliases(sql).unwrap();
        assert_eq!(result, "SELECT id, name FROM users u WHERE u.active = 1");
    }

    #[test]
    fn test_nested_alias_substitution() {
        let mut manager = AliasManager::new();
        manager.add_alias("base_columns".to_string(), "id, name".to_string());
        manager.add_alias(
            "all_columns".to_string(),
            "{{base_columns}}, email, age".to_string(),
        );

        let sql = "SELECT {{all_columns}} FROM users";
        let result = manager.substitute_aliases(sql).unwrap();
        assert_eq!(result, "SELECT id, name, email, age FROM users");
    }

    #[test]
    fn test_unresolved_alias_error() {
        let manager = AliasManager::new();
        let sql = "SELECT {{undefined}} FROM users";
        let result = manager.substitute_aliases(sql);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unresolved alias"));
    }

    #[test]
    fn test_circular_reference_detection() {
        let mut manager = AliasManager::new();
        manager.add_alias("a".to_string(), "{{b}}".to_string());
        manager.add_alias("b".to_string(), "{{a}}".to_string());

        let sql = "SELECT {{a}} FROM users";
        let result = manager.substitute_aliases(sql);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Circular alias reference")
        );
    }

    #[test]
    fn test_parse_alias_attributes() {
        let attrs: Vec<Attribute> = vec![
            parse_quote! { #[alias(all = "id, name, email")] },
            parse_quote! { #[alias(count = "SELECT COUNT(*) FROM users")] },
        ];

        let manager = AliasManager::parse_from_attributes(&attrs).unwrap();
        assert!(manager.has_aliases());

        let names = manager.get_alias_names();
        assert!(names.contains(&"all".to_string()));
        assert!(names.contains(&"count".to_string()));
    }
}
