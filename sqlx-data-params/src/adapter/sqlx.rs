use crate::{FilterValue, Params, SqlxError};
use sqlx_data_integration::{
    Arguments, BoxDynError, DB, Database, DbArgumentBuffer, DbArguments, Encode, IntoArguments,
    IsNull, Result, Type, TypeInfo,
};

// Import all types from sqlx_data_integration
use sqlx_data_integration::{
    BString, BitVec, Date, DateTime, Decimal, IpNet, IpNetwork, MacAddress, NaiveDate,
    NaiveDateTime, NaiveTime, PrimitiveDateTime, Time, Uuid,
};

// Local type alias to resolve Json generic
#[cfg(feature = "json")]
type Json = sqlx_data_integration::Json<sqlx_data_integration::JsonValue>;
#[cfg(not(feature = "json"))]
type Json = String;

fn bind_filter_values<'q, DB>(values: &'q [FilterValue]) -> Result<DbArguments<'q>>
where
    DB: Database,
    FilterValue: Encode<'q, DB> + Type<DB>,
{
    let mut args = DbArguments::default();

    for v in values {
        match v {
            FilterValue::String(s) => args.add(s).map_err(|e| {
                SqlxError::Encode(format!("Failed to bind FilterValue: {}", e).into())
            })?,

            FilterValue::Int(i) => args.add(*i).map_err(|e| {
                SqlxError::Encode(format!("Failed to bind FilterValue: {}", e).into())
            })?,

            #[cfg(any(feature = "sqlite", feature = "postgres"))]
            FilterValue::UInt(u) => args.add(*u as i64).map_err(|e| {
                SqlxError::Encode(format!("Failed to bind FilterValue: {}", e).into())
            })?,

            #[cfg(feature = "mysql")]
            FilterValue::UInt(u) => args.add(*u).map_err(|e| {
                SqlxError::Encode(format!("Failed to bind FilterValue: {}", e).into())
            })?,

            #[cfg(any(feature = "sqlite", feature = "mysql"))]
            FilterValue::Bool(b) => args.add(*b as i64).map_err(|e| {
                SqlxError::Encode(format!("Failed to bind FilterValue: {}", e).into())
            })?,

            FilterValue::Float(f) => args.add(*f).map_err(|e| {
                SqlxError::Encode(format!("Failed to bind FilterValue: {}", e).into())
            })?,

            FilterValue::Null => args.add::<Option<i32>>(None).map_err(|e| {
                SqlxError::Encode(format!("Failed to bind FilterValue: {}", e).into())
            })?,

            FilterValue::Array(_) => {
                return Err(SqlxError::Encode("Array binding not supported".into()));
            }

            // Fallback
            _ => args.add(v).map_err(|e| {
                SqlxError::Encode(format!("Failed to bind FilterValue: {}", e).into())
            })?,
        }
    }

    Ok(args)
}

/// Direct SQLx integration for FilterValue.
///
/// Allows using FilterValue directly in SQLx queries:
/// ```rust,no_run
/// use sqlx_data_params::FilterValue;
/// let filter = FilterValue::Int(42);
/// // sqlx::query("SELECT * FROM users WHERE id = ?").bind(&filter);
/// ```
#[cfg(any(feature = "mysql", feature = "postgres"))]
impl<'q> IntoArguments<'q, DB> for FilterValue {
    #[allow(clippy::expect_used)]
    fn into_arguments(self) -> DbArguments<'q> {
        bind_filter_values::<DB>(&[self]).expect("unsupported argument in filter value.")
    }
}

#[cfg(feature = "sqlite")]
impl<'q> IntoArguments<'q, DB> for FilterValue {
    #[allow(clippy::expect_used)]
    fn into_arguments(self) -> DbArguments<'q> {
        let mut args = DbArguments::default();

        let result = match self {
            FilterValue::String(s) => args.add(s),
            FilterValue::Int(i) => args.add(i),
            FilterValue::UInt(u) => args.add(u as i64),
            FilterValue::Bool(b) => args.add(b as i64),
            FilterValue::Float(f) => args.add(f),
            FilterValue::Null => args.add::<Option<i32>>(None),
            FilterValue::Array(_) => panic!("Array binding not supported"),
            _ => args.add(self),
        };

        result.expect("Failed to bind FilterValue");
        args
    }
}

impl FilterValue {
    pub fn build_arguments<'q, DB>(filters: &'q [FilterValue]) -> Result<DbArguments<'q>>
    where
        DB: Database,
        FilterValue: Encode<'q, DB> + Type<DB>,
    {
        bind_filter_values::<DB>(filters)
    }
}

impl Params {
    /// Builds arguments from multiple FilterValues for SQLx queries.
    ///
    /// Example:
    /// ```rust
    /// use sqlx_data_params::{ParamsBuilder, FilterValue, IntoParams};
    /// let params = ParamsBuilder::default()
    ///     .filter()
    ///     .eq("id", 42)
    ///     .eq("name", "example")
    ///     .done()
    ///     .build();
    /// let filters = vec![FilterValue::Int(42), FilterValue::String("example".into())];
    /// let args = params.build_arguments(&filters);
    /// // sqlx::query_scalar_with::<_, i32, _>("SELECT * FROM users WHERE id = ? AND name = ?", args);
    /// ```
    pub fn build_arguments<'q>(
        &'q self,
        bind_values: &'q [FilterValue],
    ) -> impl Fn() -> Result<DbArguments<'q>> + 'q {
        move || bind_filter_values::<DB>(bind_values)
    }
}

/// Enhanced Encode implementation with comprehensive type support for FilterValue.
///
/// Supports all SQLx-compatible types including UUID, DateTime, Decimal, Network types, etc.
/// Each type is feature-gated and falls back to appropriate compatible types when features are disabled.
impl<'q> Encode<'q, DB> for FilterValue
where
    String: Encode<'q, DB>,
    i64: Encode<'q, DB>,
    f64: Encode<'q, DB>,
    bool: Encode<'q, DB>,
    Vec<u8>: Encode<'q, DB>,
    Json: Encode<'q, DB>,
    Uuid: Encode<'q, DB>,
    DateTime: Encode<'q, DB>,
    NaiveDateTime: Encode<'q, DB>,
    NaiveDate: Encode<'q, DB>,
    NaiveTime: Encode<'q, DB>,
    PrimitiveDateTime: Encode<'q, DB>,
    Date: Encode<'q, DB>,
    Time: Encode<'q, DB>,
    Decimal: Encode<'q, DB>,
    IpNet: Encode<'q, DB>,
    IpNetwork: Encode<'q, DB>,
    BitVec: Encode<'q, DB>,
    BString: Encode<'q, DB>,
    MacAddress: Encode<'q, DB>,
{
    fn encode_by_ref(&self, args: &mut DbArgumentBuffer<'q>) -> Result<IsNull, BoxDynError> {
        match self {
            // Basic types
            FilterValue::String(s) => s.encode_by_ref(args),
            FilterValue::Int(i) => i.encode_by_ref(args),
            FilterValue::UInt(i) => (*i as i64).encode_by_ref(args),
            FilterValue::Float(f) => f.encode_by_ref(args),
            FilterValue::Bool(b) => b.encode_by_ref(args),
            FilterValue::Blob(v) => v.encode_by_ref(args),
            FilterValue::Null => Ok(IsNull::Yes),
            FilterValue::Array(_) => unreachable!("Array not supported in bind"),

            // JSON support
            #[cfg(feature = "json")]
            FilterValue::Json(s) => s.encode_by_ref(args),

            // UUID support - native types
            #[cfg(feature = "uuid")]
            FilterValue::Uuid(uuid) => uuid.encode_by_ref(args),

            // Chrono DateTime support
            #[cfg(feature = "chrono")]
            FilterValue::DateTimeChrono(dt) => dt.encode_by_ref(args),
            #[cfg(feature = "chrono")]
            FilterValue::NaiveDateTime(ndt) => ndt.encode_by_ref(args),
            #[cfg(feature = "chrono")]
            FilterValue::NaiveDate(nd) => nd.encode_by_ref(args),
            #[cfg(feature = "chrono")]
            FilterValue::NaiveTime(nt) => nt.encode_by_ref(args),

            // Time crate support
            #[cfg(all(feature = "time", not(feature = "chrono")))]
            FilterValue::OffsetDateTime(odt) => odt.encode_by_ref(args),
            #[cfg(all(feature = "time", not(feature = "chrono")))]
            FilterValue::PrimitiveDateTime(pdt) => pdt.encode_by_ref(args),
            #[cfg(all(feature = "time", not(feature = "chrono")))]
            FilterValue::Date(date) => date.encode_by_ref(args),
            #[cfg(all(feature = "time", not(feature = "chrono")))]
            FilterValue::Time(time) => time.encode_by_ref(args),

            #[cfg(all(feature = "jiff", not(feature = "chrono"), not(feature = "time")))]
            FilterValue::DateTimeJiff(dt) => dt.encode_by_ref(args),
            #[cfg(all(feature = "jiff", not(feature = "chrono"), not(feature = "time")))]
            FilterValue::NaiveDateTime(ndt) => ndt.encode_by_ref(args),
            #[cfg(all(feature = "jiff", not(feature = "chrono"), not(feature = "time")))]
            FilterValue::NaiveDate(nd) => nd.encode_by_ref(args),
            #[cfg(all(feature = "jiff", not(feature = "chrono"), not(feature = "time")))]
            FilterValue::NaiveTime(nt) => nt.encode_by_ref(args),

            // Decimal support
            #[cfg(feature = "rust_decimal")]
            FilterValue::Decimal(decimal) => decimal.encode_by_ref(args),
            #[cfg(feature = "bigdecimal")]
            FilterValue::Decimal(decimal) => decimal.encode_by_ref(args),

            // Network types
            // Note: SQLx only supports IpNet, not Ipv4Net/Ipv6Net directly
            #[cfg(feature = "ipnet")]
            FilterValue::IpNet(ipnet) => ipnet.encode_by_ref(args),
            #[cfg(feature = "ipnet")]
            FilterValue::Ipv4Net(ipv4) => IpNet::from(*ipv4).encode_by_ref(args),
            #[cfg(feature = "ipnet")]
            FilterValue::Ipv6Net(ipv6) => IpNet::from(*ipv6).encode_by_ref(args),
            #[cfg(feature = "ipnetwork")]
            FilterValue::IpNetwork(ipnet) => ipnet.encode_by_ref(args),

            // Hardware address
            #[cfg(feature = "mac_address")]
            FilterValue::MacAddress(mac) => mac.encode_by_ref(args),

            // Binary data types
            #[cfg(feature = "bit-vec")]
            FilterValue::BitVec(bits) => bits.encode_by_ref(args),
            #[cfg(feature = "bstr")]
            FilterValue::BStr(bstr) => bstr.as_bytes().encode_by_ref(args),

            // Regular expressions (stored as string)
            #[cfg(feature = "regexp")]
            FilterValue::Regex(pattern) => pattern.encode_by_ref(args),
        }
    }
}

/// Enhanced Type implementation with comprehensive type support for FilterValue.
///
/// Provides SQL type compatibility for all supported FilterValue variants including
/// UUID, DateTime, Decimal, Network types, and more through feature flags.
impl Type<DB> for FilterValue
where
    String: Type<DB>,
    i64: Type<DB>,
    f64: Type<DB>,
    bool: Type<DB>,
    Vec<u8>: Type<DB>,
    Json: Type<DB>,
    Uuid: Type<DB>,
    DateTime: Type<DB>,
    NaiveDateTime: Type<DB>,
    NaiveDate: Type<DB>,
    NaiveTime: Type<DB>,
    PrimitiveDateTime: Type<DB>,
    Date: Type<DB>,
    Time: Type<DB>,
    Decimal: Type<DB>,
    IpNet: Type<DB>,
    IpNetwork: Type<DB>,
    BitVec: Type<DB>,
    BString: Type<DB>,
    MacAddress: Type<DB>,
{
    fn type_info() -> TypeInfo {
        <i64 as Type<DB>>::type_info()
    }

    fn compatible(ty: &TypeInfo) -> bool {
        <String as Type<DB>>::compatible(ty)
            || <i64 as Type<DB>>::compatible(ty)
            || <f64 as Type<DB>>::compatible(ty)
            || <bool as Type<DB>>::compatible(ty)
            || <Vec<u8> as Type<DB>>::compatible(ty)
            || <Json as Type<DB>>::compatible(ty)
            || <Uuid as Type<DB>>::compatible(ty)
            || <DateTime as Type<DB>>::compatible(ty)
            || <NaiveDateTime as Type<DB>>::compatible(ty)
            || <NaiveDate as Type<DB>>::compatible(ty)
            || <NaiveTime as Type<DB>>::compatible(ty)
            || <PrimitiveDateTime as Type<DB>>::compatible(ty)
            || <Date as Type<DB>>::compatible(ty)
            || <Time as Type<DB>>::compatible(ty)
            || <Decimal as Type<DB>>::compatible(ty)
            || <IpNet as Type<DB>>::compatible(ty)
            || <IpNetwork as Type<DB>>::compatible(ty)
            || <BitVec as Type<DB>>::compatible(ty)
            || <BString as Type<DB>>::compatible(ty)
            || <MacAddress as Type<DB>>::compatible(ty)
    }
}
