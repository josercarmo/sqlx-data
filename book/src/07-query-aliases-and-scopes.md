# Chapter 7: Query Aliases and Scopes

SQLx-Data provides powerful query composition features through **aliases** and **scopes**, enabling you to build maintainable, reusable SQL patterns. These features help eliminate repetition and create consistent query structures across your application.

**Important**: You define your own custom alias and scope names - there are no fixed or reserved names. SQLx-Data uses whatever names you choose in your `#[alias]` and `#[scope]` definitions.

## Query Aliases

Aliases allow you to define reusable SQL fragments with **your own custom names** that can be substituted into your queries using `{{your_alias_name}}` syntax.

### Basic Alias Definition

```rust
#[repo]
#[alias(my_table = "users")]              // You choose "my_table" as the alias name
#[alias(my_columns = "id, name, email, age")]  // You choose "my_columns" as the alias name
#[alias(my_filter = "WHERE age >= ?")]    // You choose "my_filter" as the alias name
trait UserRepo {
    // Use YOUR custom alias names in queries
    #[dml("SELECT {{my_columns}} FROM {{my_table}} {{my_filter}}")]
    async fn find_adult_users(&self, min_age: u8) -> Result<Vec<User>>;
}
```

You can use any names you want:

```rust
#[repo]
#[alias(tbl = "users")]                   // Short name
#[alias(all_user_fields = "id, name, email, age")] // Descriptive name
#[alias(adult_condition = "WHERE age >= ?")]        // Business logic name
trait FlexibleUserRepo {
    #[dml("SELECT {{all_user_fields}} FROM {{tbl}} {{adult_condition}}")]
    async fn find_adults(&self, min_age: u8) -> Result<Vec<User>>;
}
```

### Complex Alias Examples

```rust
#[repo]
#[alias(
    user_columns = "id as 'id!: Id', name, email, age as 'age: u8', birth_year as 'birth_year: u16'"
)]
#[alias(user_table = "users")]
#[alias(count_query = "SELECT COUNT(*) FROM users")]
#[alias(avg_query = "SELECT AVG(age) as 'avg?: f32' FROM users")]
#[alias(values = "(?, ?, ?, ?, ?)")] // For INSERT statements
trait AdvancedUserRepo {
    // Complete query as alias
    #[dml("{{count_query}}")]
    async fn count_all_users(&self) -> Result<i64>;

    // Mixed aliases in different parts
    #[dml("SELECT {{user_columns}} FROM {{user_table}} WHERE id = ?")]
    async fn find_by_id(&self, id: i64) -> Result<Option<User>>;

    // Scalar query with alias
    #[dml("{{avg_query}}")]
    async fn average_age(&self) -> Result<Option<f32>>;

    // INSERT with alias
    #[dml("INSERT INTO {{user_table}} (id, name, email, age, birth_year) VALUES {{values}}")]
    async fn create_user(&self, id: i64, name: String, email: String, age: u8, birth_year: Option<u16>) -> Result<QueryResult>;
}
```

### File-Based Queries with Aliases

```rust
#[repo]
#[alias(user_table = "users")]
#[alias(status_filter = "WHERE active = true")]
trait UserRepo {
    // Use aliases in external SQL files
    #[dml(file = "queries/complex_user_query.sql")]
    async fn complex_user_search(&self, search_term: String) -> Result<Vec<User>>;
}
```

```sql
-- queries/complex_user_query.sql
SELECT id, name, email, age
FROM {{user_table}}
{{status_filter}}
AND (name LIKE ? OR email LIKE ?)
ORDER BY name
```

## Query Scopes

Scopes automatically add conditions to specific SQL clauses. Unlike aliases, scopes are applied automatically to ALL queries in a trait unless explicitly ignored.

### Basic Scope Definition

```rust
#[repo]
#[scope(active = "deleted_at IS NULL")]
#[scope(tenant = "tenant_id = ?")]
trait UserRepo {
    // Automatically includes: WHERE deleted_at IS NULL AND tenant_id = ?
    #[dml("SELECT * FROM users WHERE age >= ?")]
    async fn find_adult_users(&self, tenant_id: i64, min_age: u8) -> Result<Vec<User>>;

    // Also automatically includes scopes
    #[dml("SELECT * FROM users WHERE name = ?")]
    async fn find_by_name(&self, tenant_id: i64, name: String) -> Result<Option<User>>;
}
```

### Scope Targets

Scopes can target different parts of SQL queries:

```rust
#[repo]
#[alias(age_threshold = "25")]
#[scope(young = "age < {{age_threshold}}")] // Default: WHERE
#[scope(old = "age >= {{age_threshold}}")] // Default: WHERE
#[scope(has_birth_year = "birth_year IS NOT NULL")] // WHERE
#[scope(name_filter = "name LIKE '%a%'")] // WHERE
#[scope(order_name = "name ASC", target = "order_by")] // ORDER BY clause
trait ScopedUserRepo {
    // All WHERE scopes automatically applied + ORDER BY scope
    #[dml("SELECT id, name, email, age FROM users")]
    async fn find_all_users(&self) -> Result<Vec<User>>;
}
```

Generated SQL would be:
```sql
SELECT id, name, email, age FROM users
WHERE age < 25
  AND age >= 25
  AND birth_year IS NOT NULL
  AND name LIKE '%a%'
ORDER BY name ASC
```

### Scope Targets Available

```rust
#[repo]
#[scope(select_extra = "DISTINCT", target = "select")]
#[scope(active_users = "u", target = "from")] // FROM clause modification
#[scope(join_profiles = "LEFT JOIN profiles p ON u.id = p.user_id", target = "join")]
#[scope(not_deleted = "deleted_at IS NULL", target = "where")] // Default target
#[scope(group_by_dept = "department_id", target = "group_by")]
#[scope(dept_count = "COUNT(*) > 5", target = "having")]
#[scope(sort_by_name = "name ASC", target = "order_by")]
trait MultiTargetRepo {
    #[dml("SELECT * FROM users u")]
    async fn find_users(&self) -> Result<Vec<User>>;
}
```

### Ignoring Scopes

Use `#[scope_ignore]` to exclude specific scopes from individual methods:

```rust
#[repo]
#[scope(active = "deleted_at IS NULL")]
#[scope(tenant = "tenant_id = ?")]
#[scope(order_name = "name ASC", target = "order_by")]
trait UserRepo {
    // Applies all scopes
    #[dml("SELECT * FROM users")]
    async fn find_all_active_users(&self, tenant_id: i64) -> Result<Vec<User>>;

    // Ignores specific scopes
    #[scope_ignore(active)]
    #[scope_ignore(tenant)]
    #[dml("SELECT * FROM users WHERE id = ?")]
    async fn find_by_id_admin(&self, id: i64) -> Result<Option<User>>;

    // Ignores only ordering scope
    #[scope_ignore(order_name)]
    #[dml("SELECT * FROM users WHERE age > ? ORDER BY age DESC")]
    async fn find_by_age_custom_order(&self, tenant_id: i64, min_age: u8) -> Result<Vec<User>>;
}
```

## Combining Aliases and Scopes

Aliases and scopes work together seamlessly:

```rust
#[repo]
#[alias(user_columns = "id, name, email, age")]
#[alias(user_table = "users")]
#[alias(age_threshold = "25")]
#[scope(young = "age < {{age_threshold}}")] // Scope using alias
#[scope(active = "deleted_at IS NULL")]
#[scope(tenant = "tenant_id = ?")]
trait CombinedRepo {
    // Combines aliases and scopes
    #[dml("SELECT {{user_columns}} FROM {{user_table}}")]
    async fn find_young_active_users(&self, tenant_id: i64) -> Result<Vec<User>>;

    // Selective scope ignoring with aliases
    #[scope_ignore(young)]
    #[dml("SELECT {{user_columns}} FROM {{user_table}} WHERE age >= ?")]
    async fn find_adult_users(&self, tenant_id: i64, min_age: u8) -> Result<Vec<User>>;
}
```

## Processing Order

SQLx-Data processes query composition in a specific order:

1. **Alias Substitution**: All `{{alias_name}}` patterns are replaced first
2. **SQL Parsing**: The query is parsed using sqlparser
3. **Scope Application**: Scopes are applied to their target clauses
4. **Parameter Binding**: Parameters are bound to the final SQL

```rust
#[repo]
#[alias(base_query = "SELECT * FROM users WHERE created_at > ?")]
#[scope(active = "deleted_at IS NULL")]
trait ProcessingOrderRepo {
    // Processing order:
    // 1. {{base_query}} → "SELECT * FROM users WHERE created_at > ?"
    // 2. Parse SQL structure
    // 3. Apply scope: "SELECT * FROM users WHERE created_at > ? AND deleted_at IS NULL"
    // 4. Bind parameters: created_date
    #[dml("{{base_query}}")]
    async fn find_recent_users(&self, created_date: String) -> Result<Vec<User>>;
}
```

## Advanced Patterns

### Multi-level Aliases

```rust
#[repo]
#[alias(base_columns = "id, name, email")]
#[alias(extended_columns = "{{base_columns}}, age, birth_year")]
#[alias(full_user_query = "SELECT {{extended_columns}} FROM users")]
trait MultiLevelAliasRepo {
    #[dml("{{full_user_query}} WHERE id = ?")]
    async fn find_by_id(&self, id: i64) -> Result<Option<User>>;
}
```

### Conditional Scopes with Parameters

```rust
#[repo]
#[scope(tenant = "tenant_id = ?")] // Parameter will be auto-injected
#[scope(active = "status = 'active'")]
trait ConditionalRepo {
    // tenant_id parameter automatically added by scope
    #[dml("SELECT * FROM users WHERE age >= ?")]
    async fn find_adult_users(&self, tenant_id: i64, min_age: u8) -> Result<Vec<User>>;

    // Explicit ignoring when not needed
    #[scope_ignore(tenant)]
    #[dml("SELECT COUNT(*) FROM users")]
    async fn count_all_users(&self) -> Result<i64>;
}
```

### Complex Scope Combinations

```rust
#[repo]
#[alias(user_table = "users u")]
#[scope(with_profiles = "LEFT JOIN profiles p ON u.id = p.user_id", target = "join")]
#[scope(has_profile = "p.id IS NOT NULL", target = "where")]
#[scope(recent = "u.created_at > NOW() - INTERVAL '30 days'", target = "where")]
#[scope(by_activity = "u.last_login DESC", target = "order_by")]
trait ComplexScopeRepo {
    #[dml("SELECT u.*, p.bio FROM {{user_table}}")]
    async fn find_recent_users_with_profiles(&self) -> Result<Vec<(User, Option<String>)>>;

    // Ignore specific scopes for different behavior
    #[scope_ignore(has_profile)]
    #[scope_ignore(recent)]
    #[dml("SELECT u.id, u.name FROM {{user_table}}")]
    async fn find_all_users_with_profiles(&self) -> Result<Vec<(i64, String)>>;
}
```

## Best Practices

### 1. Alias Naming Conventions
```rust
#[repo]
#[alias(user_table = "users")]           // Table aliases: {entity}_table
#[alias(user_columns = "id, name, email")] // Column aliases: {entity}_columns
#[alias(where_active = "WHERE active = true")] // Condition aliases: where_{condition}
#[alias(order_name = "ORDER BY name ASC")]     // Order aliases: order_{field}
trait WellNamedRepo { }
```

### 2. Scope Organization
```rust
#[repo]
// Group related scopes together
#[scope(tenant = "tenant_id = ?")] // Security scopes first
#[scope(active = "deleted_at IS NULL")] // Business logic scopes
#[scope(default_order = "created_at DESC", target = "order_by")] // Presentation scopes last
trait OrganizedScopeRepo { }
```

### 3. Selective Scope Usage
```rust
#[repo]
#[scope(tenant = "tenant_id = ?")]
#[scope(active = "deleted_at IS NULL")]
trait SelectiveRepo {
    // Use scopes for user-facing queries
    #[dml("SELECT * FROM users")]
    async fn find_users(&self, tenant_id: i64) -> Result<Vec<User>>;

    // Ignore scopes for admin queries
    #[scope_ignore(tenant)]
    #[scope_ignore(active)]
    #[dml("SELECT * FROM users WHERE id = ?")]
    async fn admin_find_any_user(&self, id: i64) -> Result<Option<User>>;
}
```

### 4. Documentation
```rust
#[repo]
/// User repository with automatic tenant isolation and soft delete handling
#[scope(tenant = "tenant_id = ?")] // Ensures tenant isolation
#[scope(active = "deleted_at IS NULL")] // Implements soft delete
trait DocumentedRepo {
    /// Finds active users in the current tenant
    /// Automatically applies tenant and active scopes
    #[dml("SELECT * FROM users")]
    async fn find_active_users(&self, tenant_id: i64) -> Result<Vec<User>>;

    /// Admin method that bypasses tenant isolation
    /// Use with caution - no automatic scopes applied
    #[scope_ignore(tenant)]
    #[scope_ignore(active)]
    #[dml("SELECT * FROM users WHERE id = ?")]
    async fn admin_find_user(&self, id: i64) -> Result<Option<User>>;
}
```

## Performance Considerations

- **Compile-time Processing**: All alias substitution and scope application happens at compile time
- **SQL Caching**: Processed queries are cached using sqlparser for performance
- **No Runtime Overhead**: Zero runtime cost for aliases and scopes
- **Index Optimization**: Ensure scoped conditions are properly indexed

## Security Benefits

- **Consistent Security**: Scopes ensure security conditions are always applied
- **Explicit Bypassing**: `#[scope_ignore]` makes security bypass explicit and auditable
- **Parameterized Scopes**: Scope parameters are properly bound, preventing SQL injection
- **Compile-time Validation**: Invalid scope targets are caught at compile time

## Next Steps

With aliases and scopes mastered, you can build sophisticated query hierarchies that maintain consistency across your application. In the next chapter, we'll explore streaming and batch operations for handling large datasets efficiently.

Query composition with aliases and scopes provides the foundation for building maintainable, secure, and consistent database access patterns that scale with your application's complexity.