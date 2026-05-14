//! SQLx types and database configurations
//!
//! This module provides database-specific types and configurations
//! for different database backends.

#[cfg(feature = "sqlite")]
pub mod database {
    // ---------------------------------------------------------------------------------------------
    // Core database types
    // ---------------------------------------------------------------------------------------------
    pub type DB = sqlx::Sqlite;
    pub type DbArguments<'q> = <DB as sqlx::Database>::Arguments<'q>;
    pub type DbArgumentBuffer<'q> = <DB as sqlx::Database>::ArgumentBuffer<'q>;
    pub type TypeInfo = <DB as sqlx::Database>::TypeInfo;
    pub type QueryResult = sqlx::sqlite::SqliteQueryResult;
    pub type Connection = sqlx::sqlite::SqliteConnection;
    pub type Transaction<'a> = sqlx::Transaction<'a, DB>;
    pub type Pool = sqlx::Pool<DB>;
    pub type Row = <DB as sqlx::Database>::Row;

    pub type Result<T, E = sqlx::Error> = std::result::Result<T, E>;

    pub trait Executor<'c>: sqlx::Executor<'c, Database = DB> {}
    impl<'c, T> Executor<'c> for T where T: sqlx::Executor<'c, Database = DB> {}

    pub const PLACEHOLDER: &str = "$";

    pub use sqlx::{
        Arguments, Database, Encode, Error, FromRow, IntoArguments, Type,
    };

    pub type IsNull = sqlx::encode::IsNull;
    pub type BoxDynError = Box<dyn std::error::Error + Send + Sync + 'static>;

    // ---------------------------------------------------------------------------------------------
    // JSON support (stored as TEXT)
    // ---------------------------------------------------------------------------------------------
    #[cfg(feature = "json")]
    pub type Json<T> = sqlx::types::Json<T>;
    #[cfg(feature = "json")]
    pub type JsonValue = sqlx::types::JsonValue;

    #[cfg(not(feature = "json"))]
    pub type Json<T> = T;
    #[cfg(not(feature = "json"))]
    pub type JsonValue = String;

    // ---------------------------------------------------------------------------------------------
    // UUID (stored as TEXT)
    // ---------------------------------------------------------------------------------------------
    #[cfg(feature = "uuid")]
    pub type Uuid = sqlx::types::Uuid;
    #[cfg(feature = "uuid")]
    pub type Hyphenated = sqlx::types::uuid::fmt::Hyphenated;
    #[cfg(feature = "uuid")]
    pub type Simple = sqlx::types::uuid::fmt::Simple;

    #[cfg(not(feature = "uuid"))]
    pub type Uuid = String;
    #[cfg(not(feature = "uuid"))]
    pub type Hyphenated = String;
    #[cfg(not(feature = "uuid"))]
    pub type Simple = String;

    // ---------------------------------------------------------------------------------------------
    // Date / Time (stored as TEXT or INTEGER – no native semantics)
    // ---------------------------------------------------------------------------------------------
    #[cfg(feature = "chrono")]
    pub type DateTime = sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>;
    #[cfg(feature = "chrono")]
    pub type NaiveDateTime = sqlx::types::chrono::NaiveDateTime;
    #[cfg(feature = "chrono")]
    pub type NaiveDate = sqlx::types::chrono::NaiveDate;
    #[cfg(feature = "chrono")]
    pub type NaiveTime = sqlx::types::chrono::NaiveTime;
    #[cfg(feature = "chrono")]
    pub type PrimitiveDateTime = sqlx::types::chrono::NaiveDateTime;
    #[cfg(feature = "chrono")]
    pub type Date = sqlx::types::chrono::NaiveDate;
    #[cfg(feature = "chrono")]
    pub type Time = sqlx::types::chrono::NaiveTime;

    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type DateTime = sqlx::types::time::OffsetDateTime;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type OffsetDateTime = sqlx::types::time::OffsetDateTime;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type PrimitiveDateTime = sqlx::types::time::PrimitiveDateTime;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type Date = sqlx::types::time::Date;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type Time = sqlx::types::time::Time;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type NaiveDateTime = sqlx::types::time::PrimitiveDateTime;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type NaiveDate = sqlx::types::time::Date;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type NaiveTime = sqlx::types::time::Time;

    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    pub type DateTime = jiff_sqlx::Timestamp;
    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    pub type NaiveDateTime = jiff_sqlx::DateTime;
    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    pub type NaiveDate = jiff_sqlx::Date;
    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    pub type NaiveTime = jiff_sqlx::Time;
    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    pub type PrimitiveDateTime = jiff_sqlx::DateTime;
    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    pub type Date = jiff_sqlx::Date;
    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    pub type Time = jiff_sqlx::Time;

    // Fallbacks (plain TEXT)
    #[cfg(not(any(feature = "chrono", feature = "time", feature = "jiff")))]
    pub type DateTime = String;
    #[cfg(not(any(feature = "chrono", feature = "time", feature = "jiff")))]
    pub type NaiveDateTime = String;
    #[cfg(not(any(feature = "chrono", feature = "time", feature = "jiff")))]
    pub type NaiveDate = String;
    #[cfg(not(any(feature = "chrono", feature = "time", feature = "jiff")))]
    pub type NaiveTime = String;
    #[cfg(not(any(feature = "chrono", feature = "time", feature = "jiff")))]
    pub type PrimitiveDateTime = String;
    #[cfg(not(any(feature = "chrono", feature = "time", feature = "jiff")))]
    pub type Date = String;
    #[cfg(not(any(feature = "chrono", feature = "time", feature = "jiff")))]
    pub type Time = String;

    // ---------------------------------------------------------------------------------------------
    // Decimal (stored as TEXT or REAL — precision not guaranteed)
    // ---------------------------------------------------------------------------------------------
    //pub type DecimalType = sqlx::types::Decimal; SQLite does not support native decimal
    #[cfg(feature = "rust_decimal")]
    pub type Decimal = String;
    #[cfg(all(feature = "bigdecimal", not(feature = "rust_decimal")))]
    pub type Decimal = sqlx::types::Decimal;
    #[cfg(not(any(feature = "rust_decimal", feature = "bigdecimal")))]
    pub type Decimal = String;

    // ---------------------------------------------------------------------------------------------
    // Network types (stored as TEXT — no semantic comparison)
    // ---------------------------------------------------------------------------------------------
    #[cfg(feature = "ipnet")]
    pub type IpNet = String;
    #[cfg(feature = "ipnet")]
    pub type Ipv4Net = String;
    #[cfg(feature = "ipnet")]
    pub type Ipv6Net = String;

    #[cfg(feature = "ipnetwork")]
    pub type IpNetwork = String;

    #[cfg(not(feature = "ipnet"))]
    pub type IpNet = String;
    #[cfg(not(feature = "ipnet"))]
    pub type Ipv4Net = String;
    #[cfg(not(feature = "ipnet"))]
    pub type Ipv6Net = String;
    #[cfg(not(feature = "ipnetwork"))]
    pub type IpNetwork = String;

    // ---------------------------------------------------------------------------------------------
    // Binary / misc
    // ---------------------------------------------------------------------------------------------
    #[cfg(feature = "bit-vec")]
    pub type BitVec = Vec<u8>;
    #[cfg(not(feature = "bit-vec"))]
    pub type BitVec = Vec<u8>;

    #[cfg(feature = "bstr")]
    pub type BString = sqlx::types::bstr::BString;
    #[cfg(not(feature = "bstr"))]
    pub type BString = Vec<u8>;

    #[cfg(feature = "mac_address")]
    pub type MacAddress = sqlx::types::mac_address::MacAddress;
    #[cfg(not(feature = "mac_address"))]
    pub type MacAddress = Vec<u8>;
}


#[cfg(feature = "postgres")]
pub mod database {
    pub type DB = sqlx::Postgres;
    pub type DbArguments<'q> = <DB as sqlx::Database>::Arguments<'q>;
    pub type DbArgumentBuffer<'q> = <DB as sqlx::Database>::ArgumentBuffer<'q>;
    pub type TypeInfo = <DB as sqlx::Database>::TypeInfo;
    pub type QueryResult = sqlx::postgres::PgQueryResult;
    pub type Connection = sqlx::postgres::PgConnection;
    pub type Transaction<'a> = sqlx::Transaction<'a, sqlx::Postgres>;
    pub type Pool = sqlx::Pool<sqlx::Postgres>;
    pub type Row = <sqlx::Postgres as sqlx::Database>::Row;
    pub type Result<T, E = sqlx::Error> = std::result::Result<T, E>;
    pub trait Executor<'c>: sqlx::Executor<'c, Database = DB> {}
    impl<'c, T> Executor<'c> for T where T: sqlx::Executor<'c, Database = DB> {}

    pub const PLACEHOLDER: &str = "$";

    pub type IsNull = sqlx::encode::IsNull;
    pub type BoxDynError = Box<dyn std::error::Error + Send + Sync + 'static>;

    pub use sqlx::{Arguments, Database, Encode, Error, IntoArguments, Type, FromRow};

    // ---------------- JSON ----------------
    #[cfg(feature = "json")]
    pub type JsonValue = sqlx::types::JsonValue;
    #[cfg(feature = "json")]
    pub type Json<T> = sqlx::types::Json<T>;

    #[cfg(not(feature = "json"))]
    pub type JsonValue = String;
    #[cfg(not(feature = "json"))]
    pub type Json<T> = T;

    // ---------------- UUID ----------------
    #[cfg(feature = "uuid")]
    pub type Uuid = sqlx::types::Uuid;
    #[cfg(feature = "uuid")]
    pub type Hyphenated = sqlx::types::uuid::fmt::Hyphenated;
    #[cfg(feature = "uuid")]
    pub type Simple = sqlx::types::uuid::fmt::Simple;

    #[cfg(not(feature = "uuid"))]
    pub type Uuid = String;
    #[cfg(not(feature = "uuid"))]
    pub type Hyphenated = String;
    #[cfg(not(feature = "uuid"))]
    pub type Simple = String;

    // ---------------- DATE / TIME ----------------
    #[cfg(feature = "chrono")]
    pub type DateTime = sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>;
    #[cfg(feature = "chrono")]
    pub type NaiveDateTime = sqlx::types::chrono::NaiveDateTime;
    #[cfg(feature = "chrono")]
    pub type NaiveDate = sqlx::types::chrono::NaiveDate;
    #[cfg(feature = "chrono")]
    pub type NaiveTime = sqlx::types::chrono::NaiveTime;
    #[cfg(feature = "chrono")]
    pub type PrimitiveDateTime = sqlx::types::chrono::NaiveDateTime;
    #[cfg(feature = "chrono")]
    pub type Date = sqlx::types::chrono::NaiveDate;
    #[cfg(feature = "chrono")]
    pub type Time = sqlx::types::chrono::NaiveTime;

    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type DateTime = sqlx::types::time::OffsetDateTime;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type OffsetDateTime = sqlx::types::time::OffsetDateTime;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type PrimitiveDateTime = sqlx::types::time::PrimitiveDateTime;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type Date = sqlx::types::time::Date;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type Time = sqlx::types::time::Time;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type NaiveDateTime = sqlx::types::time::PrimitiveDateTime;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type NaiveDate = sqlx::types::time::Date;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type NaiveTime = sqlx::types::time::Time;

    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    pub type DateTime = jiff_sqlx::Timestamp;
    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    pub type NaiveDateTime = jiff_sqlx::DateTime;
    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    pub type NaiveDate = jiff_sqlx::Date;
    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    pub type NaiveTime = jiff_sqlx::Time;
    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    pub type PrimitiveDateTime = jiff_sqlx::DateTime;
    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    pub type Date = jiff_sqlx::Date;
    #[cfg(all(
        feature = "jiff",
        not(feature = "chrono"),
        not(feature = "time")
    ))]
    pub type Time = jiff_sqlx::Time;

    // Fallbacks (plain TEXT)
    #[cfg(not(any(feature = "chrono", feature = "time", feature = "jiff")))]
    pub type DateTime = String;
    #[cfg(not(any(feature = "chrono", feature = "time", feature = "jiff")))]
    pub type NaiveDateTime = String;
    #[cfg(not(any(feature = "chrono", feature = "time", feature = "jiff")))]
    pub type NaiveDate = String;
    #[cfg(not(any(feature = "chrono", feature = "time", feature = "jiff")))]
    pub type NaiveTime = String;
    #[cfg(not(any(feature = "chrono", feature = "time", feature = "jiff")))]
    pub type PrimitiveDateTime = String;
    #[cfg(not(any(feature = "chrono", feature = "time", feature = "jiff")))]
    pub type Date = String;
    #[cfg(not(any(feature = "chrono", feature = "time", feature = "jiff")))]
    pub type Time = String;

    // ---------------- DECIMAL ----------------
    #[cfg(feature = "rust_decimal")]
    pub type Decimal = sqlx::types::Decimal;
    #[cfg(all(feature = "bigdecimal", not(feature = "rust_decimal")))]
    pub type Decimal = sqlx::types::BigDecimal;
    #[cfg(not(any(feature = "rust_decimal", feature = "bigdecimal")))]
    pub type Decimal = String;

    // ---------------- NETWORK ----------------
    #[cfg(feature = "ipnet")]
    pub type IpNet = sqlx::types::ipnet::IpNet;
    #[cfg(feature = "ipnet")]
    pub type Ipv4Net = sqlx::types::ipnet::Ipv4Net;
    #[cfg(feature = "ipnet")]
    pub type Ipv6Net = sqlx::types::ipnet::Ipv6Net;

    #[cfg(feature = "ipnetwork")]
    pub type IpNetwork = sqlx::types::ipnetwork::IpNetwork;

    #[cfg(not(feature = "ipnet"))]
    pub type IpNet = String;
    #[cfg(not(feature = "ipnet"))]
    pub type Ipv4Net = String;
    #[cfg(not(feature = "ipnet"))]
    pub type Ipv6Net = String;
    #[cfg(not(feature = "ipnetwork"))]
    pub type IpNetwork = String;

    // ---------------- BINARY ----------------
    #[cfg(feature = "bit-vec")]
    pub type BitVec = sqlx::types::BitVec;
    #[cfg(feature = "bstr")]
    pub type BString = sqlx::types::bstr::BString;

    #[cfg(not(feature = "bit-vec"))]
    pub type BitVec = Vec<u8>;
    #[cfg(not(feature = "bstr"))]
    pub type BString = Vec<u8>;

    // ---------------- MAC ----------------
    #[cfg(feature = "mac_address")]
    pub type MacAddress = sqlx::types::mac_address::MacAddress;
    #[cfg(not(feature = "mac_address"))]
    pub type MacAddress = Vec<u8>;
}

#[cfg(feature = "mysql")]
pub mod database {
    pub type DB = sqlx::MySql;
    pub type DbArguments<'q> = <DB as sqlx::Database>::Arguments<'q>;
    pub type DbArgumentBuffer<'q> = <DB as sqlx::Database>::ArgumentBuffer<'q>;
    pub type TypeInfo = <DB as sqlx::Database>::TypeInfo;
    pub type QueryResult = sqlx::mysql::MySqlQueryResult;
    pub type Connection = sqlx::mysql::MySqlConnection;
    pub type Transaction<'a> = sqlx::Transaction<'a, sqlx::MySql>;
    pub type Pool = sqlx::Pool<sqlx::MySql>;
    pub type Row = <sqlx::MySql as sqlx::Database>::Row;
    pub type Result<T, E = sqlx::Error> = std::result::Result<T, E>;

    pub trait Executor<'c>: sqlx::Executor<'c, Database = DB> {}
    impl<'c, T> Executor<'c> for T where T: sqlx::Executor<'c, Database = DB> {}

    pub const PLACEHOLDER: &str = "?";

    pub type IsNull = sqlx::encode::IsNull;
    pub type BoxDynError = Box<dyn std::error::Error + Send + Sync + 'static>;

    pub use sqlx::{Arguments, Database, Encode, Error, IntoArguments, Type, FromRow};

    // ---------------- JSON ----------------
    #[cfg(feature = "json")]
    pub type JsonValue = sqlx::types::JsonValue;
    #[cfg(feature = "json")]
    pub type Json<T> = sqlx::types::Json<T>;

    #[cfg(not(feature = "json"))]
    pub type JsonValue = String;
    #[cfg(not(feature = "json"))]
    pub type Json<T> = T;

    // ---------------- UUID ----------------
    #[cfg(feature = "uuid")]
    pub type Uuid = sqlx::types::Uuid;
    #[cfg(feature = "uuid")]
    pub type Hyphenated = sqlx::types::uuid::fmt::Hyphenated;
    #[cfg(feature = "uuid")]
    pub type Simple = sqlx::types::uuid::fmt::Simple;

    #[cfg(not(feature = "uuid"))]
    pub type Uuid = String;
    #[cfg(not(feature = "uuid"))]
    pub type Hyphenated = String;
    #[cfg(not(feature = "uuid"))]
    pub type Simple = String;

    // ---------------- DATE / TIME ----------------
    #[cfg(feature = "chrono")]
    pub type DateTime = sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>;
    #[cfg(feature = "chrono")]
    pub type NaiveDateTime = sqlx::types::chrono::NaiveDateTime;
    #[cfg(feature = "chrono")]
    pub type NaiveDate = sqlx::types::chrono::NaiveDate;
    #[cfg(feature = "chrono")]
    pub type NaiveTime = sqlx::types::chrono::NaiveTime;
    #[cfg(feature = "chrono")]
    pub type PrimitiveDateTime = sqlx::types::chrono::NaiveDateTime;
    #[cfg(feature = "chrono")]
    pub type Date = sqlx::types::chrono::NaiveDate;
    #[cfg(feature = "chrono")]
    pub type Time = sqlx::types::chrono::NaiveTime;

    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type DateTime = sqlx::types::time::OffsetDateTime;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type PrimitiveDateTime = sqlx::types::time::PrimitiveDateTime;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type Date = sqlx::types::time::Date;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type Time = sqlx::types::time::Time;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type NaiveDateTime = sqlx::types::time::PrimitiveDateTime;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type NaiveDate = sqlx::types::time::Date;
    #[cfg(all(feature = "time", not(feature = "chrono")))]
    pub type NaiveTime = sqlx::types::time::Time;

    // Fallbacks (plain TEXT)
    #[cfg(not(any(feature = "chrono", feature = "time")))]
    pub type DateTime = String;
    #[cfg(not(any(feature = "chrono", feature = "time")))]
    pub type NaiveDateTime = String;
    #[cfg(not(any(feature = "chrono", feature = "time")))]
    pub type NaiveDate = String;
    #[cfg(all(not(feature = "chrono"), not(feature = "time")))]
    pub type NaiveTime = String;
    #[cfg(not(any(feature = "chrono", feature = "time")))]
    pub type PrimitiveDateTime = String;
    #[cfg(not(any(feature = "chrono", feature = "time")))]
    pub type Date = String;
    #[cfg(not(any(feature = "chrono", feature = "time")))]
    pub type Time = String;

    // ---------------- DECIMAL ----------------
    #[cfg(feature = "rust_decimal")]
    pub type Decimal = sqlx::types::Decimal;
    #[cfg(all(feature = "bigdecimal", not(feature = "rust_decimal")))]
    pub type Decimal = sqlx::types::BigDecimal;
    #[cfg(not(any(feature = "rust_decimal", feature = "bigdecimal")))]
    pub type Decimal = String;

    // ---------------- NETWORK ----------------
    #[cfg(feature = "ipnet")]
    pub type IpNet = sqlx::types::ipnet::IpNet;
    #[cfg(feature = "ipnetwork")]
    pub type IpNetwork = sqlx::types::ipnetwork::IpNetwork;

    #[cfg(not(feature = "ipnet"))]
    pub type IpNet = String;
    #[cfg(not(feature = "ipnet"))]
    pub type Ipv4Net = String;
    #[cfg(not(feature = "ipnet"))]
    pub type Ipv6Net = String;
    #[cfg(not(feature = "ipnetwork"))]
    pub type IpNetwork = String;

    // ---------------- BINARY ----------------
    #[cfg(feature = "bit-vec")]
    pub type BitVec = sqlx::types::BitVec;
    #[cfg(feature = "bstr")]
    pub type BString = sqlx::types::bstr::BString;

    #[cfg(not(feature = "bit-vec"))]
    pub type BitVec = Vec<u8>;
    #[cfg(not(feature = "bstr"))]
    pub type BString = Vec<u8>;

    // ---------------- MAC ----------------
    #[cfg(feature = "mac_address")]
    pub type MacAddress = sqlx::types::mac_address::MacAddress;
    #[cfg(not(feature = "mac_address"))]
    pub type MacAddress = Vec<u8>;
}
