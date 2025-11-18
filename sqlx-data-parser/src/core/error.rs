#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParserError {
    #[error("error occurred while parsing SQL: {0}")]
    ParseSql(String),

    #[error("{0}")]
    Validation(String),

    #[error("{0}")]
    InvalidArgument(String),
}

impl ParserError {
    pub fn parse_sql(msg: &str) -> Self {
        ParserError::ParseSql(msg.to_string())
    }

    pub fn error(msg: &str) -> Self {
        ParserError::Validation(msg.to_string())
    }

    pub fn invalid_argument(msg: &str) -> Self {
        ParserError::InvalidArgument(msg.to_string())
    }
}
