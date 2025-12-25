# Chapter 6: Pagination Strategies

SQLx-Data provides three sophisticated pagination strategies to handle different performance and use-case requirements: **Serial**, **Slice**, and **Cursor** pagination. Each strategy is optimized for specific scenarios and provides different trade-offs between simplicity, performance, and consistency.

SQLx-Data uses **sqlparser**, one of the best SQL query builders available, which completely avoids weak string concatenation found in other projects. Everything is done at compile time when possible. Dynamic queries are built at runtime only once and cached, but sqlparser itself is extremely fast.

## Overview of Pagination Types

### Serial Pagination
- **Use case**: Traditional page-based navigation with total counts
- **Performance**: Requires COUNT query for metadata
- **Query Strategy**: Executes exactly LIMIT items (no +1 optimization)
- **Best for**: User interfaces with page numbers and total counts

### Slice Pagination
- **Use case**: Simple "next/previous" navigation with optional counts
- **Performance**: Fastest - uses LIMIT+1 optimization to detect next page
- **Query Strategy**: Executes LIMIT+1 items, removes extra item if found
- **Best for**: APIs and mobile apps with simple navigation

### Cursor Pagination
- **Use case**: Real-time feeds and infinite scroll
- **Performance**: Excellent for large datasets, stable across data changes
- **Query Strategy**: Uses WHERE conditions based on sort fields
- **Best for**: Social feeds, chat applications, and streaming data

## Serial Pagination

Serial pagination provides traditional page-based navigation with complete metadata.

### Basic Serial Pagination

```rust
use sqlx_data::{Serial, IntoParams, ParamsBuilder, SerialParams, SerialBuilder};

#[repo]
trait UserRepo {
    #[dml("SELECT id, name, age FROM users")]
    async fn find_all(&self, params: impl IntoParams) -> Result<Serial<User>>;

    // Direct SerialParams usage
    #[dml("SELECT id, name, age FROM users WHERE age >= ?")]
    async fn find_adults(&self, min_age: i64, params: SerialParams) -> Result<Serial<User>>;
}
```

### Multiple Ways to Create Serial Pagination

#### 1. Using ParamsBuilder (Most Flexible)
```rust
let params = ParamsBuilder::new()
    .serial()
        .page(1, 10)  // Page 1, 10 items per page
        .done()
    .sort()
        .asc("name")  // Sort by name ascending
        .done()
    .filter()
        .gte("age", 18)  // Age >= 18
        .done()
    .build();

let page = repo.find_all(params).await?;
```

#### 2. Using SerialParams Directly
```rust
let params = SerialParams::new(1, 10);  // Page 1, 10 items
let page = repo.find_adults(25, params).await?;
```

#### 3. Using SerialBuilder Standalone
```rust
let params = SerialBuilder::new()
    .page(2, 5)  // Page 2, 5 items
    .build();

let page = repo.find_adults(18, params).await?;
```

#### 4. SerialParams with Options
```rust
let params = SerialParams::new(1, 10);
let page = repo.find_adults(21, params).await?;

println!("Page: {} of {}", page.page, page.total_pages);
println!("Total items: {}", page.total_items);
```

### Serial Response Structure

```rust
pub struct Serial<T> {
    pub data: Vec<T>,           // The actual data
    pub page: u32,              // Current page number (1-based)
    pub size: u32,              // Items per page
    pub total_items: i64,       // Total number of items
    pub total_pages: u32,       // Total number of pages
}
```

The Serial response provides complete pagination metadata by executing a separate COUNT query.

## Slice Pagination

Slice pagination provides lightweight pagination without counting total items.

### Basic Slice Pagination

```rust
use sqlx_data::{Slice, IntoParams, ParamsBuilder, SliceParams, SliceBuilder};

#[repo]
trait UserRepo {
    #[dml("SELECT id, name, age FROM users")]
    async fn find_all(&self, params: impl IntoParams) -> Result<Slice<User>>;

    // Direct SliceParams usage
    #[dml("SELECT name, age FROM users WHERE name LIKE ?")]
    async fn find_by_name_pattern(
        &self,
        pattern: String,
        params: SliceParams,
    ) -> Result<Slice<(String, i64)>>;
}
```

### Multiple Ways to Create Slice Pagination

#### 1. Using ParamsBuilder (Most Flexible)
```rust
let params = ParamsBuilder::new()
    .slice()
        .page(1, 5)   // Page 1, 5 items per page
        .done()
    .sort()
        .desc("id")   // Sort by ID descending
        .done()
    .filter()
        .like("name", "A%")  // Names starting with 'A'
        .done()
    .build();

let slice = repo.find_all(params).await?;
```

#### 2. Using SliceParams Directly
```rust
let params = SliceParams::new(1, 10);  // Page 1, 10 items
let slice = repo.find_by_name_pattern("Alice%".to_string(), params).await?;
```

#### 3. Using SliceBuilder Standalone
```rust
let params = SliceBuilder::new()
    .page(2, 5)  // Page 2, 5 items
    .build();

let slice = repo.find_by_name_pattern("Bob%".to_string(), params).await?;
```

#### 4. SliceParams with Total Count Control
```rust
// Default - no total count (fastest)
let params = SliceParams::new(1, 10);

// Enable total count (defeats performance purpose, but sometimes needed)
let params_with_count = SliceParams::new(1, 10)
    .with_disable_total_count(false);

let slice = repo.find_by_name_pattern("Charlie%".to_string(), params_with_count).await?;

if let Some(total) = slice.total_items {
    println!("Total items: {}", total);
}
```

### Slice Response Structure

```rust
pub struct Slice<T> {
    pub data: Vec<T>,              // The actual data
    pub page: u32,                 // Current page number (1-based)
    pub size: u32,                 // Page size
    pub has_next: bool,            // Whether there's a next page (LIMIT+1 detection)
    pub has_previous: bool,        // Whether there's a previous page
    pub total_items: Option<i64>,  // Optional total count (rarely used)
}
```

### Optional Total Count

Slice pagination can optionally provide total counts in special cases:

```rust
// Enable total count (rarely needed - defeats the performance purpose)
let params = ParamsBuilder::new()
    .slice()
        .page(1, 10)
        .enable_total_count()  // Enables the COUNT query
        .done()
    .build();

let result = repo.find_all(params).await?;
if let Some(total) = result.total_items {
    println!("Total items: {}", total);
}
```

The LIMIT+1 strategy works by requesting one extra item beyond the page size, then removing it if found - this efficiently determines if there's a next page without a separate COUNT query.

## Cursor Pagination

Cursor pagination provides stable pagination for real-time data and large datasets.

### Setting Up Cursor Pagination

First, implement the `CursorSecureExtract` trait for your model:

```rust
use sqlx_data::{Cursor, CursorSecureExtract, CursorValue, CursorData, FilterValue, CursorError};

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
}

impl CursorSecureExtract for User {
    fn extract_whitelisted_fields(&self, fields: &[String]) -> Result<Vec<CursorValue>> {
        let mut values = Vec::with_capacity(fields.len());
        for field in fields {
            match field.as_str() {
                "id" => values.push(self.id.into()),
                "name" => values.push(self.name.clone().into()),
                _ => return Err(CursorError::invalid_field(field.clone()).into()),
            }
        }
        Ok(values)
    }

    fn encode(cursor: &CursorData) -> Result<String> {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as BASE64};
        let json_bytes = serde_json::to_vec(&cursor)
            .map_err(|e| CursorError::encode_error(format!("JSON error: {}", e)))?;
        Ok(BASE64.encode(json_bytes))
    }

    fn decode(encoded: &str) -> Result<Vec<FilterValue>> {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as BASE64};
        let bytes = BASE64.decode(encoded)
            .map_err(|e| CursorError::decode_error(format!("Base64 error: {}", e)))?;

        let cursor: CursorData = serde_json::from_slice(&bytes)
            .map_err(|e| CursorError::decode_error(format!("JSON error: {}", e)))?;

        let filter_values: Vec<FilterValue> = cursor.entries.into_iter()
            .map(|entry| match entry.value {
                CursorValue::Int(v) => FilterValue::Int(v),
                CursorValue::String(v) => v.into(),
                CursorValue::Float(v) => FilterValue::Float(v),
                CursorValue::Bool(v) => FilterValue::Bool(v),
                CursorValue::UInt(v) => FilterValue::UInt(v),
            }).collect();

        Ok(filter_values)
    }
}
```

### Basic Cursor Pagination

```rust
use sqlx_data::{Cursor, IntoParams, ParamsBuilder, CursorParams, CursorBuilder};

#[repo]
trait UserRepo {
    #[dml("SELECT id, name FROM users")]
    async fn find_all(&self, params: impl IntoParams) -> Result<Cursor<User>>;

    // Direct CursorParams usage (less common)
    #[dml("SELECT id, name FROM users WHERE status = ?")]
    async fn find_active(&self, status: String, params: CursorParams) -> Result<Cursor<User>>;
}
```

### Multiple Ways to Create Cursor Pagination

All cursor pagination requires the `CursorSecureExtract` trait implementation (shown earlier).

### Practical Cursor Navigation Example

```rust
async fn navigate_with_cursors(repo: &impl UserRepo) -> Result<()> {
    // First page - start from beginning
    let params = ParamsBuilder::new()
        .cursor()
            .first_page()  // Explicit first page
            .done()
        .sort()
            .asc("id")
            .done()
        .limit(5)
        .build();

    let page1 = repo.find_all(params).await?;
    println!("Page 1: {} items", page1.data.len());

    // Navigate to next page using encoded cursor
    if let Some(next_cursor_token) = &page1.next_cursor {
        let next_params = ParamsBuilder::new()
            .cursor()
                .next_cursor::<User>(next_cursor_token)
                .done()
            .sort()
                .asc("id")
                .done()
            .limit(5)
            .build();

        let page2 = repo.find_all(next_params).await?;
        println!("Page 2: {} items", page2.data.len());

        // Navigate back using previous cursor
        if let Some(prev_cursor_token) = &page2.prev_cursor {
            let prev_params = ParamsBuilder::new()
                .cursor()
                    .prev_cursor::<User>(prev_cursor_token)
                    .done()
                .sort()
                    .asc("id")
                    .done()
                .limit(5)
                .build();

            let back_to_page1 = repo.find_all(prev_params).await?;
            println!("Back to page 1: {} items", back_to_page1.data.len());
        }
    }

    Ok(())
}
```

### Cursor Response Structure

```rust
pub struct Cursor<T> {
    pub data: Vec<T>,                // The actual data
    pub per_page: u32,               // Items per page
    pub has_next: bool,              // Whether there's a next page
    pub has_prev: bool,              // Whether there's a previous page
    pub next_cursor: Option<String>, // Encoded cursor for next page
    pub prev_cursor: Option<String>, // Encoded cursor for previous page
}
```

### Multiple Ways to Use Cursor Pagination

#### 1. First Page (Empty Cursor)
```rust
let params = ParamsBuilder::new()
    .cursor()
        .first_page()  // Explicitly request first page
        .done()
    .sort()
        .asc("id")
        .done()
    .limit(10)
    .build();
```

#### 2. After a Specific Value
```rust
let params = ParamsBuilder::new()
    .cursor()
        .after(5)  // Get records after ID 5
        .done()
    .sort()
        .asc("id")  // Must match cursor field
        .done()
    .limit(10)
    .build();
```

#### 3. Before a Specific Value (Reverse)
```rust
let params = ParamsBuilder::new()
    .cursor()
        .before(15)  // Get records before ID 15
        .done()
    .sort()
        .asc("id")
        .done()
    .limit(10)
    .build();
```

#### 4. From Encoded Cursor (Next Page)
```rust
// Using encoded cursor from previous response
let params = ParamsBuilder::new()
    .cursor()
        .next_cursor::<User>(&encoded_cursor_token)
        .done()
    .sort()
        .asc("id")
        .done()
    .limit(10)
    .build();
```

#### 5. From Encoded Cursor (Previous Page)
```rust
// Using encoded cursor for reverse navigation
let params = ParamsBuilder::new()
    .cursor()
        .prev_cursor::<User>(&encoded_cursor_token)
        .done()
    .sort()
        .asc("id")
        .done()
    .limit(10)
    .build();
```

#### 6. Composite Cursors (Multiple Sort Fields)
```rust
let params = ParamsBuilder::new()
    .cursor()
        .after(("2023-01-01".to_string(), 100))  // After date and ID
        .and_field(42)  // Additional field
        .done()
    .sort()
        .asc("created_at")
        .asc("id")  // Multiple sort fields
        .done()
    .limit(10)
    .build();
```

## Standalone Builders and Advanced Usage

SQLx-Data provides multiple ways to build parameters beyond ParamsBuilder:

### Standalone FilterBuilder

```rust
use sqlx_data::FilterBuilder;

// Create filters independently
let filters = FilterBuilder::new()
    .r#in("id", vec![1, 3, 5])     // IN clause
    .gte("age", 18)                 // Age >= 18
    .like("name", "Alice")          // LIKE with auto-escaping
    .is_not_null("email")           // NOT NULL check
    .build();

// Use with repository method that accepts FilterParams
let result = repo.find_by_filters(filters).await?;
```

### Standalone SortBuilder

```rust
use sqlx_data::SortBuilder;

// Create sorting independently
let sorting = SortBuilder::new()
    .asc("name")                    // Safe compile-time field
    .desc("created_at")
    .nulls_last()                   // NULL handling for last field
    .build();

// Convert to Params if needed
let params = sorting.map(|s| {
    Params {
        sort_by: Some(s),
        ..Default::default()
    }
}).unwrap_or_default();
```

### Standalone SearchBuilder

```rust
use sqlx_data::SearchBuilder;

// Create search independently
let search = SearchBuilder::new()
    .query("Alice")
    .fields(vec!["name", "email", "username"])
    .case_sensitive(false)
    .exact(false)                   // Fuzzy matching
    .build();

// Convert to Params
let params = Params {
    search: Some(search),
    ..Default::default()
};
```

### Unsafe Sorting with Runtime Validation

```rust
// For dynamic column names from user input
let user_sort_field = "name"; // From user input

let sorting = SortBuilder::new()
    .with_allowed_columns(&["id", "name", "email", "created_at"])
    .asc_unsafe(user_sort_field)   // Runtime validation against whitelist
    .build();

if let Some(sort_params) = sorting {
    // Validate fields before use
    sort_params.validate_fields()?;
}
```

## Combining with Filters and Sorting

All pagination types work seamlessly with filtering and sorting:

### Complex Filtering Example

```rust
async fn find_users_with_filters(repo: &impl UserRepo) -> Result<()> {
    let params = ParamsBuilder::new()
        .serial()
            .page(1, 20)
            .done()
        .filter()
            .gte("age", 18)           // Age >= 18
            .contains("name", "Alice") // Name contains "Alice"
            .r#in("status", vec!["active", "premium"])
            .between("created_at", "2023-01-01", "2023-12-31")
            .done()
        .sort()
            .asc("name")
            .desc("age")              // Secondary sort by age desc
            .done()
        .build();

    let result = repo.find_all(params).await?;
    println!("Found {} adult users with 'Alice' in name", result.data.len());
    Ok(())
}
```

### Search Integration

```rust
async fn search_users(repo: &impl UserRepo, query: String) -> Result<()> {
    let params = ParamsBuilder::new()
        .slice()
            .page(1, 10)
            .done()
        .search()
            .query(&query)
            .fields(vec!["name", "email"])  // Search in name and email fields
            .case_sensitive(false)
            .done()
        .sort()
            .asc("name")
            .done()
        .build();

    let results = repo.find_all(params).await?;
    println!("Search '{}' returned {} results", query, results.data.len());
    Ok(())
}
```

## Performance Characteristics

### Serial Pagination
- **Query Count**: 2 (data + count)
- **Performance**: Slower due to COUNT query
- **Memory**: Low - only loads current page
- **Consistency**: Excellent - stable page numbers

### Slice Pagination
- **Query Count**: 1 (data only with LIMIT + 1)
- **Performance**: Fastest
- **Memory**: Low - only loads current page + 1
- **Consistency**: Good - may show duplicates during concurrent writes

### Cursor Pagination
- **Query Count**: 1 (data only)
- **Performance**: Excellent for large datasets
- **Memory**: Low - only loads current page
- **Consistency**: Best - stable across concurrent changes

## Choosing the Right Strategy

### Use Serial When:
- Building traditional web UIs with page numbers
- Users need to jump to specific pages
- Total count information is important
- Dataset is relatively small (< 100K records)

### Use Slice When:
- Building mobile apps or APIs
- Simple next/previous navigation is sufficient
- Performance is critical
- Total count is not needed

### Use Cursor When:
- Building real-time feeds or infinite scroll
- Dataset is very large (> 1M records)
- Stability across concurrent writes is critical
- Building social media feeds or chat applications

## Advanced Patterns

### Hybrid Approach

```rust
// Use slice for fast browsing, serial when count is needed
async fn smart_pagination(
    repo: &impl UserRepo,
    show_totals: bool
) -> Result<Box<dyn std::fmt::Debug>> {
    if show_totals {
        let params = ParamsBuilder::new().serial().page(1, 20).done().build();
        let result = repo.find_all(params).await?;
        Ok(Box::new(result))
    } else {
        let params = ParamsBuilder::new().slice().page(1, 20).done().build();
        let result = repo.find_all_slice(params).await?;
        Ok(Box::new(result))
    }
}
```

### Cursor with Multiple Sort Fields

```rust
async fn cursor_multi_sort(repo: &impl UserRepo) -> Result<()> {
    let params = ParamsBuilder::new()
        .cursor()
            .after(("2023-01-01".to_string(), 100))  // After date and ID
            .done()
        .sort()
            .asc("created_at")
            .asc("id")      // Secondary sort for uniqueness
            .done()
        .limit(25)
        .build();

    let page = repo.find_all(params).await?;
    println!("Multi-field cursor returned {} items", page.data.len());
    Ok(())
}
```

## Security: SQL Injection and Cursor Injection Prevention

SQLx-Data implements multiple layers of security to prevent SQL injection and cursor injection attacks.

### SQL Injection Prevention

#### 1. Safe vs Unsafe Operations

**Safe Operations (Compile-time validated):**
```rust
// Safe LIKE - automatically escapes % and _ characters
.like("name", "user_input")     // Treats % and _ as literal characters

// Safe sorting - only accepts &'static str
.asc("id")                      // Compile-time field validation
.desc("created_at")            // No dynamic strings allowed
```

**Unsafe Operations (Runtime validated against whitelist):**
```rust
// Unsafe LIKE - allows wildcards (controlled environments only)
.like_pattern("name", "admin_%") // Intentional wildcard usage

// Unsafe sorting - accepts dynamic strings but validates against whitelist
.with_allowed_columns(&["id", "name", "email", "created_at"])
.asc_unsafe(user_input_field)   // Runtime validation against whitelist
```

#### 2. Automatic Escaping

```rust
// Automatic escaping in FilterBuilder
let params = FilterBuilder::new()
    .like("filename", "report_2023.pdf")     // Escapes _ automatically
    .contains("description", "100% safe")     // Escapes % automatically
    .build();

// vs Unsafe (only for controlled wildcards)
let params = FilterBuilder::new()
    .like_pattern("path", "/admin/%")        // Intentional wildcard
    .build();
```

#### 3. Whitelist Validation

```rust
// Dynamic sort fields require whitelisting
let user_sort_field = get_sort_from_user(); // Potentially malicious input

let params = ParamsBuilder::new()
    .sort()
        .with_allowed_columns(&["id", "name", "email", "created_at"]) // Explicit whitelist
        .asc_unsafe(user_sort_field)    // Validates against whitelist at runtime
        .done()
    .build();

// This will fail if user_sort_field is not in the whitelist
// Preventing SQL injection via ORDER BY clause
```

### Cursor Injection Prevention

#### 1. Size Limits (DoS Prevention)
```rust
// Maximum 10 fields allowed in cursors to prevent DoS attacks
const MAX_CURSOR_FIELDS: usize = 10;

// This will be rejected:
let oversized_cursor = (0..20)
    .map(|i| FilterValue::Int(i))
    .collect();
// Error: "Cursor too large: 20 fields (max 10)"
```

#### 2. Type Safety in Cursor Values
```rust
// Cursors use strongly-typed FilterValue enum
pub enum FilterValue {
    String(Cow<'static, str>),
    Int(i64),
    Float(f64),
    Bool(bool),
    // ... other safe types
}

// No raw SQL strings allowed in cursor values
```

#### 3. Secure Encoding/Decoding
```rust
impl CursorSecureExtract for User {
    // Whitelist approach - only allowed fields can be encoded
    fn extract_whitelisted_fields(&self, fields: &[String]) -> Result<Vec<CursorValue>> {
        for field in fields {
            match field.as_str() {
                "id" => values.push(self.id.into()),
                "name" => values.push(self.name.clone().into()),
                _ => return Err(CursorError::invalid_field(field.clone())), // Reject unknown fields
            }
        }
    }

    // Base64 encoding with JSON structure
    fn encode(cursor: &CursorData) -> Result<String> {
        let json_bytes = serde_json::to_vec(&cursor)?;
        Ok(BASE64.encode(json_bytes))  // Safe encoding
    }

    // Safe decoding with validation
    fn decode(encoded: &str) -> Result<Vec<FilterValue>> {
        let bytes = BASE64.decode(encoded)?;           // Validate Base64
        let cursor: CursorData = serde_json::from_slice(&bytes)?; // Validate JSON
        // Convert to FilterValue (type-safe)
    }
}
```

### Runtime Validation

All dynamic parameters are validated before SQL execution:

```rust
// validate_fields() is called automatically on all dynamic queries
pub fn validate_fields(params: &Params) -> Result<()> {
    // 1. Validate unsafe sort fields against whitelist
    if let Some(sort_params) = &params.sort_by {
        if sort_params.has_unsafe_fields() {
            sort_params.validate_fields()?;  // SQL injection prevention
        }
    }

    // 2. Validate cursor size to prevent DoS
    if let Some(Pagination::Cursor(cursor)) = &params.pagination {
        if cursor.values().len() > MAX_CURSOR_FIELDS {
            return Err("Cursor too large");  // DoS prevention
        }
    }

    Ok(())
}
```

### Security Best Practices

#### ✅ Safe Patterns
```rust
// Always prefer safe operations
.like("field", user_input)      // Auto-escaping
.asc("static_field")           // Compile-time validation
.contains("field", user_query) // Safe substring search

// Use whitelisting for dynamic operations
.with_allowed_columns(&["id", "name"])
.asc_unsafe(dynamic_field)     // Runtime validation
```

#### ❌ Dangerous Patterns to Avoid
```rust
// DON'T: Never use like_pattern with user input
.like_pattern("field", user_input)  // Wildcard injection risk

// DON'T: Never use asc_unsafe without whitelist
.asc_unsafe(user_field)        // No validation = SQL injection

// DON'T: Never bypass validation
// All validation is automatic and cannot be disabled
```

### Security Features Summary

1. **Compile-time Safety**: Static strings only for safe operations
2. **Automatic Escaping**: LIKE operations escape % and _ by default
3. **Whitelist Validation**: Dynamic fields validated against explicit whitelists
4. **Type Safety**: Strongly-typed cursor values prevent injection
5. **Size Limits**: Cursor field limits prevent DoS attacks
6. **Runtime Validation**: All parameters validated before SQL execution

SQLx-Data's security model follows the principle of "secure by default" - safe operations require no special handling, while potentially dangerous operations require explicit opt-in with validation.

## Best Practices

1. **Always include sorting** - pagination without deterministic ordering can produce inconsistent results
2. **Use unique fields for cursor sorting** - prefer ID or timestamp fields for cursor pagination
3. **Consider index optimization** - ensure your sort fields are properly indexed
4. **Handle empty results gracefully** - all pagination types can return empty data arrays
5. **Validate page parameters** - check for reasonable page sizes and page numbers
6. **Cache total counts** - for Serial pagination, consider caching COUNT results when appropriate
7. **Prefer safe operations** - use compile-time validated fields when possible
8. **Whitelist dynamic fields** - always validate user input against known-good field lists

## Next Steps

With comprehensive pagination strategies and security measures understood, you're ready to explore SQLx-Data's advanced query composition features. In the next chapter, we'll dive into aliases and scopes - powerful tools for building reusable, composable queries that keep your code DRY and maintainable.