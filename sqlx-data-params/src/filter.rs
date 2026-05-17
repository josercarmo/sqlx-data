use std::borrow::Cow;
use crate::{IntoParams, Params};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilterOperator {
    Eq,
    Ne,
    Gt,
    Lt,
    Gte,
    Lte,
    /// Safe LIKE operator - automatically escapes special characters (% and _) to treat them as literals.
    /// Use this for user input to prevent wildcard injection.
    /// Example: searching "test_file%" will match exactly "test_file%", not use % and _ as wildcards.
    Like,
    /// Case-insensitive LIKE operator - automatically escapes special characters.
    /// Similar to Like but case-insensitive. Safe for user input.
    ILike,
    /// Unsafe LIKE operator - allows intentional wildcards (% and _) in patterns.
    /// Use only with controlled input where you intentionally want wildcard behavior.
    /// Example: "user_%" will match "user_123", "user_abc", etc.
    /// WARNING: Never use with direct user input due to wildcard injection risk.
    UnsafeLike,
    In,
    NotIn,
    IsNull,
    IsNotNull,
    Between,
    /// Safe contains operator - wraps value in % wildcards and escapes special characters.
    /// Ideal for substring searches. Always safe for user input.
    Contains,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FilterValue {
    String(Cow<'static, str>),
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    Array(Vec<FilterValue>),
    // DateTime(String), // RFC3339 - fallback string format
    // Date(String), // YYYY-MM-DD - fallback string format

    // JSON support with native types
    #[cfg(feature = "json")]
    Json(sqlx_data_integration::JsonValue),

    // UUID support - native SQLx re-exported types
    #[cfg(feature = "uuid")]
    Uuid(sqlx_data_integration::Uuid),
    
    // DateTime support with SQLx re-exported types
    #[cfg(feature = "chrono")]
    DateTimeChrono(sqlx_data_integration::DateTime),
    #[cfg(feature = "chrono")]
    NaiveDateTime(sqlx_data_integration::NaiveDateTime),
    #[cfg(feature = "chrono")]
    NaiveDate(sqlx_data_integration::NaiveDate),
    #[cfg(feature = "chrono")]
    NaiveTime(sqlx_data_integration::NaiveTime),

    #[cfg(all(feature = "time", not(feature = "chrono")))]
    OffsetDateTime(sqlx_data_integration::DateTime),
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    PrimitiveDateTime(sqlx_data_integration::PrimitiveDateTime),
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    Date(sqlx_data_integration::Date),
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    Time(sqlx_data_integration::Time),

    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    DateTimeJiff(sqlx_data_integration::DateTime),
    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    NaiveDateTime(sqlx_data_integration::NaiveDateTime),
    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    NaiveDate(sqlx_data_integration::NaiveDate),
    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    NaiveTime(sqlx_data_integration::NaiveTime),

    // Decimal/Money support with SQLx re-exported types
    #[cfg(feature = "rust_decimal")]
    Decimal(sqlx_data_integration::Decimal),
    #[cfg(feature = "bigdecimal")]
    Decimal(sqlx_data_integration::Decimal),

    // Network types with SQLx re-exported types
    #[cfg(feature = "ipnet")]
    IpNet(sqlx_data_integration::IpNet),
    #[cfg(feature = "ipnet")]
    Ipv4Net(sqlx_data_integration::Ipv4Net),
    #[cfg(feature = "ipnet")]
    Ipv6Net(sqlx_data_integration::Ipv6Net),

    #[cfg(feature = "ipnetwork")]
    IpNetwork(sqlx_data_integration::IpNetwork),

    // Hardware address with SQLx re-exported type
    #[cfg(feature = "mac_address")]
    MacAddress(sqlx_data_integration::MacAddress),

    // Binary data types with SQLx re-exported types
    #[cfg(feature = "bit-vec")]
    BitVec(sqlx_data_integration::BitVec),
    #[cfg(feature = "bstr")]
    BStr(sqlx_data_integration::BString),

    // Regular expressions
    #[cfg(feature = "regexp")]
    Regex(String), // Regex pattern as string for serialization

    Blob(Vec<u8>),
    Null,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Filter {
    pub field: String,
    pub operator: FilterOperator,
    pub value: FilterValue,
    /// Negates the filter operation. Always operates on the last applied filter.
    /// Applies to: Like, UnsafeLike, ILike, In, Between.
    /// Examples:
    /// - Like with not=true becomes NOT LIKE
    /// - In with not=true becomes NOT IN
    /// - Between with not=true becomes NOT BETWEEN
    ///
    /// Usage: .like("name", "pattern").not() // Creates NOT LIKE
    /// Note: IsNull/IsNotNull don't use this field as they are already explicit.
    pub not: bool,
}

impl Filter {
    pub fn new(
        field: impl Into<String>,
        operator: FilterOperator,
        value: impl Into<FilterValue>,
    ) -> Self {
        Self {
            field: field.into(),
            operator,
            value: value.into(),
            not: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct FilterParams {
    pub filters: Vec<Filter>,
}

impl IntoParams for FilterParams {
    fn into_params(self) -> Params {
        Params {
            filters: Some(self),
            search: None,
            sort_by: None,
            pagination: None,
            limit: None,
            offset: None,
        }
    }
}

// Implement From for common Rust types

impl<'a, T> From<&'a [T]> for FilterValue
where
    T: Into<FilterValue> + Copy,
{
    fn from(slice: &'a [T]) -> Self {
        FilterValue::Array(slice.iter().copied().map(Into::into).collect())
    }
}

impl From<String> for FilterValue {
    fn from(value: String) -> Self {
        FilterValue::String(Cow::Owned(value))
    }
}

impl From<&String> for FilterValue {
    fn from(value: &String) -> Self {
        FilterValue::String(Cow::Owned(value.to_owned()))
    }
}

impl From<&str> for FilterValue {
    fn from(value: &str) -> Self {
        FilterValue::String(Cow::Owned(value.to_owned()))
    }
}

impl From<i8> for FilterValue {
    fn from(value: i8) -> Self {
        FilterValue::Int(value as i64)
    }
}

impl From<i16> for FilterValue {
    fn from(value: i16) -> Self {
        FilterValue::Int(value as i64)
    }
}

impl From<i32> for FilterValue {
    fn from(value: i32) -> Self {
        FilterValue::Int(value as i64)
    }
}

impl From<i64> for FilterValue {
    fn from(value: i64) -> Self {
        FilterValue::Int(value)
    }
}

impl From<u8> for FilterValue {
    fn from(value: u8) -> Self {
        FilterValue::UInt(value as u64)
    }
}

impl From<u16> for FilterValue {
    fn from(value: u16) -> Self {
        FilterValue::UInt(value as u64)
    }
}

impl From<u32> for FilterValue {
    fn from(value: u32) -> Self {
        FilterValue::UInt(value as u64)
    }
}

impl From<u64> for FilterValue {
    fn from(value: u64) -> Self {
        FilterValue::UInt(value)
    }
}

impl From<f32> for FilterValue {
    fn from(value: f32) -> Self {
        FilterValue::Float(value as f64)
    }
}

impl From<f64> for FilterValue {
    fn from(value: f64) -> Self {
        FilterValue::Float(value)
    }
}

impl From<isize> for FilterValue {
    fn from(value: isize) -> Self {
        FilterValue::Int(value as i64)
    }
}

impl From<usize> for FilterValue {
    fn from(value: usize) -> Self {
        FilterValue::UInt(value as u64)
    }
}

impl From<bool> for FilterValue {
    fn from(value: bool) -> Self {
        FilterValue::Bool(value)
    }
}

impl<T> From<Option<T>> for FilterValue
where
    T: Into<FilterValue>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => v.into(),
            None => FilterValue::Null,
        }
    }
}

impl<T> From<Vec<T>> for FilterValue
where
    T: Into<FilterValue>,
{
    fn from(vec: Vec<T>) -> Self {
        FilterValue::Array(vec.into_iter().map(Into::into).collect())
    }
}

impl<T, const N: usize> From<[T; N]> for FilterValue
where
    T: Into<FilterValue> + Copy,
{
    fn from(array: [T; N]) -> Self {
        FilterValue::Array(array.iter().copied().map(Into::into).collect())
    }
}

// From implementations for all feature-gated types

// UUID support with SQLx re-exported types
#[cfg(feature = "uuid")]
impl From<sqlx_data_integration::Uuid> for FilterValue {
    fn from(value: sqlx_data_integration::Uuid) -> Self {
        FilterValue::Uuid(value)
    }
}

// Chrono DateTime support with SQLx re-exported types
#[cfg(feature = "chrono")]
impl From<sqlx_data_integration::DateTime> for FilterValue {
    fn from(value: sqlx_data_integration::DateTime) -> Self {
        FilterValue::DateTimeChrono(value)
    }
}

#[cfg(feature = "chrono")]
impl From<sqlx_data_integration::NaiveDateTime> for FilterValue {
    fn from(value: sqlx_data_integration::NaiveDateTime) -> Self {
        FilterValue::NaiveDateTime(value)
    }
}

#[cfg(feature = "chrono")]
impl From<sqlx_data_integration::NaiveDate> for FilterValue {
    fn from(value: sqlx_data_integration::NaiveDate) -> Self {
        FilterValue::NaiveDate(value)
    }
}

#[cfg(feature = "chrono")]
impl From<sqlx_data_integration::NaiveTime> for FilterValue {
    fn from(value: sqlx_data_integration::NaiveTime) -> Self {
        FilterValue::NaiveTime(value)
    }
}

// Time crate support with SQLx re-exported types
#[cfg(all(feature = "time", not(feature = "chrono")))]
impl From<sqlx_data_integration::DateTime> for FilterValue {
    fn from(value: sqlx_data_integration::DateTime) -> Self {
        FilterValue::OffsetDateTime(value)
    }
}

#[cfg(all(feature = "time", not(feature = "chrono")))]
impl From<sqlx_data_integration::PrimitiveDateTime> for FilterValue {
    fn from(value: sqlx_data_integration::PrimitiveDateTime) -> Self {
        FilterValue::PrimitiveDateTime(value)
    }
}

#[cfg(all(feature = "time", not(feature = "chrono")))]
impl From<sqlx_data_integration::Date> for FilterValue {
    fn from(value: sqlx_data_integration::Date) -> Self {
        FilterValue::Date(value)
    }
}

#[cfg(all(feature = "time", not(feature = "chrono")))]
impl From<sqlx_data_integration::Time> for FilterValue {
    fn from(value: sqlx_data_integration::Time) -> Self {
        FilterValue::Time(value)
    }
}

#[cfg(all(
    feature = "jiff",
    not(feature = "chrono"),
    not(feature = "time")
))]
impl From<sqlx_data_integration::DateTime> for FilterValue {
    fn from(value: sqlx_data_integration::DateTime) -> Self {
        FilterValue::DateTimeJiff(value)
    }
}

#[cfg(all(
    feature = "jiff",
    not(feature = "chrono"),
    not(feature = "time")
))]
impl From<sqlx_data_integration::NaiveDateTime> for FilterValue {
    fn from(value: sqlx_data_integration::NaiveDateTime) -> Self {
        FilterValue::NaiveDateTime(value)
    }
}

#[cfg(all(
    feature = "jiff",
    not(feature = "chrono"),
    not(feature = "time")
))]
impl From<sqlx_data_integration::NaiveDate> for FilterValue {
    fn from(value: sqlx_data_integration::NaiveDate) -> Self {
        FilterValue::NaiveDate(value)
    }
}

#[cfg(all(
    feature = "jiff",
    not(feature = "chrono"),
    not(feature = "time")
))]
impl From<sqlx_data_integration::NaiveTime> for FilterValue {
    fn from(value: sqlx_data_integration::NaiveTime) -> Self {
        FilterValue::NaiveTime(value)
    }
}

// Decimal support with SQLx re-exported types
#[cfg(all(feature = "rust_decimal", not(feature = "sqlite")))]
impl From<sqlx_data_integration::Decimal> for FilterValue {
    fn from(value: sqlx_data_integration::Decimal) -> Self {
        FilterValue::RustDecimal(value)
    }
}

#[cfg(all(feature = "bigdecimal", not(feature = "sqlite")))]
impl From<sqlx_data_integration::Decimal> for FilterValue {
    fn from(value: sqlx_data_integration::Decimal) -> Self {
        FilterValue::Decimal(value)
    }
}

// Network types with SQLx re-exported types
#[cfg(all(feature = "ipnet", not(feature = "sqlite")))]
impl From<sqlx_data_integration::IpNet> for FilterValue {
    fn from(value: sqlx_data_integration::IpNet) -> Self {
        FilterValue::IpNet(value)
    }
}

#[cfg(all(feature = "ipnet", not(feature = "sqlite")))]
impl From<sqlx_data_integration::Ipv4Net> for FilterValue {
    fn from(value: sqlx_data_integration::Ipv4Net) -> Self {
        FilterValue::Ipv4Net(value)
    }
}

#[cfg(all(feature = "ipnet", not(feature = "sqlite")))]
impl From<sqlx_data_integration::Ipv6Net> for FilterValue {
    fn from(value: sqlx_data_integration::Ipv6Net) -> Self {
        FilterValue::Ipv6Net(value)
    }
}

#[cfg(all(feature = "ipnetwork", not(feature = "sqlite")))]
impl From<sqlx_data_integration::IpNetwork> for FilterValue {
    fn from(value: sqlx_data_integration::IpNetwork) -> Self {
        FilterValue::IpNetwork(value)
    }
}

// MAC address with SQLx re-exported type
#[cfg(feature = "mac_address")]
impl From<sqlx_data_integration::MacAddress> for FilterValue {
    fn from(value: sqlx_data_integration::MacAddress) -> Self {
        FilterValue::MacAddress(value)
    }
}

// Binary data types with SQLx re-exported types
#[cfg(all(feature = "bit-vec", not(feature = "sqlite")))]
impl From<sqlx_data_integration::BitVec> for FilterValue {
    fn from(value: sqlx_data_integration::BitVec) -> Self {
        FilterValue::BitVec(value)
    }
}

#[cfg(feature = "bstr")]
impl From<sqlx_data_integration::BString> for FilterValue {
    fn from(value: sqlx_data_integration::BString) -> Self {
        FilterValue::BStr(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_value_from_primitives() {
        assert_eq!(
            FilterValue::from("hello"),
            FilterValue::String(Cow::Owned("hello".to_string()))
        );
        assert_eq!(FilterValue::from("hello".to_string()), "hello".into());
        assert_eq!(FilterValue::from(42i64), FilterValue::Int(42));
        assert_eq!(FilterValue::from(42u32), FilterValue::UInt(42));
        assert_eq!(FilterValue::from(1.5f64), FilterValue::Float(1.5f64));
        assert_eq!(FilterValue::from(true), FilterValue::Bool(true));
    }

    #[test]
    fn test_filter_value_from_option() {
        assert_eq!(FilterValue::from(Some("value")), "value".into());
        assert_eq!(FilterValue::from(None::<String>), FilterValue::Null);
        assert_eq!(FilterValue::from(Some(100i32)), FilterValue::Int(100));
        assert_eq!(FilterValue::from(None::<i32>), FilterValue::Null);
    }

    #[test]
    fn test_filter_value_from_vec() {
        let vec_str: Vec<String> = vec!["admin".to_string(), "user".to_string()];
        let expected = FilterValue::Array(vec!["admin".into(), "user".into()]);
        assert_eq!(FilterValue::from(vec_str), expected);

        let vec_int: Vec<i32> = vec![1, 2, 3];
        let expected_int = FilterValue::Array(vec![
            FilterValue::Int(1),
            FilterValue::Int(2),
            FilterValue::Int(3),
        ]);
        assert_eq!(FilterValue::from(vec_int), expected_int);
    }

    #[test]
    fn test_filter_value_from_slice() {
        let value: FilterValue = ["admin", "moderator"].into();
        let expected = FilterValue::Array(vec!["admin".into(), "moderator".into()]);
        assert_eq!(value, expected);

        let slice: &[i64] = &[10, 20, 30];
        let value: FilterValue = slice.into();
        let expected = FilterValue::Array(vec![
            FilterValue::Int(10),
            FilterValue::Int(20),
            FilterValue::Int(30),
        ]);
        assert_eq!(value, expected);
    }

    #[test]
    fn test_filter_new_with_vec_and_slice() {
        let filter1 = Filter::new(
            "role",
            FilterOperator::In,
            vec!["admin".to_string(), "user".to_string()],
        );
        assert_eq!(filter1.field, "role");
        assert_eq!(filter1.operator, FilterOperator::In);
        assert_eq!(
            filter1.value,
            FilterValue::Array(vec!["admin".into(), "user".into(),])
        );
        assert!(!filter1.not);

        let filter2 = Filter::new("status", FilterOperator::In, ["active", "pending"]);
        assert_eq!(filter2.field, "status");
        assert_eq!(filter2.operator, FilterOperator::In);
        assert_eq!(
            filter2.value,
            FilterValue::Array(vec!["active".into(), "pending".into(),])
        );
    }

    #[test]
    fn test_filter_with_mixed_option_in_array() {
        let values: Vec<Option<&str>> = vec![Some("yes"), None, Some("maybe")];
        let filter = Filter::new("answer", FilterOperator::In, values);

        assert_eq!(
            filter.value,
            FilterValue::Array(vec!["yes".into(), FilterValue::Null, "maybe".into(),])
        );
    }

}
