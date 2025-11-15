//! # sqlx-data
//!
//! A powerful SQLx extension providing automatic parameter binding, dynamic SQL generation,
//! and result parsing with trait-based repositories and advanced pagination support.
//!
//! ## Features
//!
//! - **🔧 Automatic Parameter Binding**: No more manual `bind()` calls
//! - **📄 Multiple Pagination Types**: Serial (traditional), Slice (offset-based), Cursor (keyset-based)
//! - **🔍 Dynamic Filtering & Search**: Type-safe filters with fluent API
//! - **⚡ Zero-cost Abstractions**: Compile-time code generation
//! - **🛡️ Type Safety**: Full Rust type checking for SQL queries
//! - **🎯 Trait-based Repositories**: Clean, testable architecture
//!
//! ```

// ====================================================
// PUBLIC API - This is what users should import
// ====================================================

// Core macros for repository definition
pub use sqlx_data_macros::{dml, repo, repo as repository, generate_versions};

pub mod macros {
    pub use sqlx_data_macros::{dml, repo, repo as repository, generate_versions};
}

// Organized module re-exports
pub mod database {
    // Re-export database types from sqlx-data-integration
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    pub use sqlx_data_integration::*;

    // Re-export dynamic functions from sqlx-data-parser when database features are enabled
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    pub use sqlx_data_parser::{build_count_query_from_sql, build_dynamic_sql, validate_fields};
}

#[allow(unused_imports)]
pub use database::*;

// Re-export Stream as a type alias for convenience
//pub use futures::Stream;
//pub use futures::StreamExt;

/// Convenience prelude that includes commonly used types and traits
pub mod prelude {
    pub use crate::{
        Cursor,
        CursorError,
        CursorSecureExtract,
        CursorValue,
        IntoParams,
        // Params
        ParamsBuilder,
        // Pagination
        Serial,
        Slice,
        // Macros
        dml,
        repo,
        repository,
    };
    //pub use futures::Stream; 
    //pub use futures::StreamExt;

    // Core database types (conditionally available)
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    pub use crate::database::{Connection, DB, Executor, Pool, QueryResult, Result, Transaction};
    #[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
    pub use sqlx_data_parser::{build_count_query_from_sql, build_dynamic_sql, validate_fields};
}

pub mod pagination {
    pub use sqlx_data_params::{Cursor, Serial, Slice};
}
pub use pagination::*;

pub mod params {
    pub use sqlx_data_params::{
        CursorBuilder,
        CursorParams,
        FilterBuilder,
        FilterParams,
        // Traits
        IntoParams,
        // Other types
        LimitParam,
        OffsetParam,
        Pagination,
        // Params types
        Params,
        // Builders
        ParamsBuilder,
        SearchBuilder,
        SearchParams,
        SerialBuilder,
        SerialParams,
        SliceBuilder,
        SliceParams,
        SortBuilder,
        SortingParams,
    };
}

pub use params::*;

pub mod filters {
    pub use sqlx_data_params::{
        // Cursor types
        CursorData,
        CursorDirection,
        CursorError,
        CursorSecureExtract,
        CursorValue,
        // Filter types
        Filter,
        FilterOperator,
        FilterValue,
        NullOrdering,
        // Sort types
        Sort,
        SortDirection,
    };
}

pub use filters::*;

// Macro for compile-time only SQL validation
#[macro_export]
macro_rules! compile_time_only {
    ($($code:tt)*) => {
        // Only include in debug builds for compile-time validation
        #[cfg(debug_assertions)]
        {
            // Check that the code compiles without executing it
            if false {
                $($code)*;
            }
        }
    };
}