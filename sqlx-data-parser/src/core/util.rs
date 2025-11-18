/// Get the placeholder prefix for the current database
/// Returns "$" for PostgreSQL/SQLite and "?" for MySQL
#[inline]
pub fn placeholder_prefix() -> &'static str {
    #[cfg(feature = "mysql")]
    {
        "?"
    }
    #[cfg(any(feature = "postgres", feature = "sqlite"))]
    {
        "$"
    }
    #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
    {
        "$"
    }
}
