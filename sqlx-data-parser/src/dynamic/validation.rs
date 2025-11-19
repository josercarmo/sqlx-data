#![cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]

//! Runtime validation module for dynamic SQL generation
//!
//! This module provides validation functions that are called by generated code
//! to ensure safe execution of dynamic SQL operations.

use super::error::{Result, SqlxError};
use sqlx_data_params::{Pagination, Params};

/// Maximum number of cursor fields allowed to prevent DoS attacks
/// This limit helps mitigate risks associated with excessively large
/// cursor payloads that could lead to performance degradation.
const MAX_CURSOR_FIELDS: usize = 10;

/// Validates all parameters before dynamic SQL execution
///
/// This function is called by generated code to perform runtime validations
/// that cannot be checked at compile time. Currently validates:
/// - Unsafe sort fields against whitelists
/// - Cursor field count to prevent DoS attacks
///
/// Future validations can be added here without changing generated code.
pub fn validate_fields(params: &Params) -> Result<()> {
    // Validate unsafe sort fields if present
    // Security: Prevent SQL injection via unsafe sort fields
    if let Some(sort_params) = &params.sort_by {
        if sort_params.has_unsafe_fields() {
            sort_params.validate_fields().map_err(|msg| {
                SqlxError::InvalidArgument(format!("Sort validation failed: {}", msg))
            })?;
        }
    }

    // Security: Prevent DoS attacks via oversized cursors
    if let Some(Pagination::Cursor(cursor_params)) = &params.pagination {
        if cursor_params.values().len() > MAX_CURSOR_FIELDS {
            return Err(SqlxError::InvalidArgument(format!(
                "Cursor too large: {} fields (max {})",
                cursor_params.values().len(),
                MAX_CURSOR_FIELDS
            )));
        }
    }

    // Future validations can be added here:
    // - Filter value validation
    // - Search parameter validation
    // - SQL injection prevention

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx_data_params::{CursorDirection, CursorParams, FilterValue, IntoParams, ParamsBuilder};

    #[test]
    fn test_validate_fields_with_safe_sorts() {
        let params = ParamsBuilder::new()
            .sort()
            .asc("id")
            .desc("name")
            .done()
            .build();

        // Should pass without error (no unsafe fields)
        assert!(validate_fields(&params).is_ok());
    }

    #[test]
    fn test_validate_fields_with_valid_unsafe_sorts() {
        let params = ParamsBuilder::new()
            .sort()
            .with_allowed_columns(&["id", "name", "email"])
            .asc_unsafe("name".to_string())
            .desc_unsafe("id".to_string())
            .done()
            .build();

        // Should pass (unsafe fields are in whitelist)
        assert!(validate_fields(&params).is_ok());
    }

    #[test]
    fn test_validate_fields_with_invalid_unsafe_sorts() {
        let params = ParamsBuilder::new()
            .sort()
            .with_allowed_columns(&["id", "name"])
            .asc_unsafe("malicious_field".to_string()) // Not in whitelist
            .done()
            .build();

        // Should fail (unsafe field not in whitelist)
        let result = validate_fields(&params);
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Sort validation failed"));
        assert!(error_msg.contains("malicious_field"));
    }

    #[test]
    fn test_validate_fields_with_no_sorts() {
        let params = ParamsBuilder::new()
            .filter()
            .eq("name", "test".to_string())
            .done()
            .build();

        // Should pass (no sorts to validate)
        assert!(validate_fields(&params).is_ok());
    }

    #[test]
    fn test_validate_fields_with_mixed_safe_unsafe() {
        let params = ParamsBuilder::new()
            .sort()
            .asc("id") // safe
            .with_allowed_columns(&["name", "email"])
            .desc_unsafe("name".to_string()) // unsafe but valid
            .done()
            .build();

        // Should pass (unsafe field is in whitelist)
        assert!(validate_fields(&params).is_ok());
    }

    #[test]
    fn test_validate_fields_with_valid_cursor() {
        // Create cursor with few fields (under limit)
        let values = vec![
            FilterValue::Int(1),
            FilterValue::String("test".into()),
            FilterValue::Int(100),
        ];
        let cursor_params = CursorParams::from_values(values, CursorDirection::After);

        let params = cursor_params.into_params();

        // Should pass (cursor under field limit)
        assert!(validate_fields(&params).is_ok());
    }

    #[test]
    fn test_validate_fields_with_oversized_cursor() {
        // Create cursor with too many fields (over limit)
        let values = (0..15) // More than MAX_CURSOR_FIELDS (10)
            .map(|i| FilterValue::Int(i))
            .collect();
        let cursor_params = CursorParams::from_values(values, CursorDirection::After);

        let params = cursor_params.into_params();

        // Should fail (cursor exceeds field limit)
        let result = validate_fields(&params);
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Cursor too large"));
        assert!(error_msg.contains("15 fields"));
        assert!(error_msg.contains("max 10"));

        // Should fail (cursor exceeds field limit)
        let result = validate_fields(&params);
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Cursor too large"));
        assert!(error_msg.contains("15 fields"));
        assert!(error_msg.contains("max 10"));
    }

    #[test]
    fn test_validate_fields_with_cursor_at_limit() {
        // Create cursor with exactly the maximum allowed fields
        let values = (0..MAX_CURSOR_FIELDS)
            .map(|i| FilterValue::Int(i as i64))
            .collect();
        let cursor_params = CursorParams::from_values(values, CursorDirection::After);

        let params = cursor_params.into_params();

        // Should pass (cursor at exact limit)
        assert!(validate_fields(&params).is_ok());
    }

    #[test]
    fn test_validate_fields_with_empty_cursor() {
        // Create cursor with no fields
        let cursor_params = CursorParams::from_values(vec![], CursorDirection::After);

        let params = cursor_params.into_params();

        // Should pass (empty cursor is valid)
        assert!(validate_fields(&params).is_ok());

        // Should pass (empty cursor is valid)
        assert!(validate_fields(&params).is_ok());
    }

    #[test]
    fn test_decode_rejects_oversized_cursor() {
        // Create a cursor that would decode to more than MAX_CURSOR_FIELDS
        let oversized_values = (0..20)  // More than MAX_CURSOR_FIELDS (10)
            .map(|i| FilterValue::Int(i))
            .collect();

        // This simulates what happens when next_cursor/prev_cursor decodes a cursor
        // The decoded values get converted to CursorParams and then validated
        let cursor_params = CursorParams::from_values(oversized_values, CursorDirection::After);

        let params = cursor_params.into_params();

        // Should fail during validation (oversized cursor from decode)
        let result = validate_fields(&params);
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Cursor too large"));
        assert!(error_msg.contains("20 fields"));
        assert!(error_msg.contains("max 10"));
    }
}
