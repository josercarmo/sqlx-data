use crate::constants::pagination;
use crate::fetch::{generate_fetch_call_expr, generate_pool_expr};
use crate::type_system::{FetchMethod, QueryType, ReturnType};
use syn::spanned::Spanned;
use syn::{GenericArgument, PathArguments, Type};

/// Main type analyzer
pub struct TypeAnalyzer;

impl TypeAnalyzer {
    pub fn analyze_type(ty: &Type) -> Result<ReturnType, syn::Error> {
        match ty {
            Type::Path(tp) => Self::analyze_path(tp),
            Type::Tuple(t) => Ok(Self::analyze_tuple(t)),
            Type::ImplTrait(impl_trait) => Self::analyze_impl_trait(impl_trait),
            _ => Ok(ReturnType::Unknown {
                name: "UnknownType".to_string(),
            }),
        }
    }

    fn analyze_path(tp: &syn::TypePath) -> Result<ReturnType, syn::Error> {
        let path = &tp.path;

        if path.segments.is_empty() {
            return Ok(ReturnType::Unknown {
                name: "empty_path".to_string(),
            });
        }

        let Some(last) = path.segments.last() else {
            return Ok(ReturnType::Unknown {
                name: "EmptyPath".to_string(),
            });
        };

        match &last.arguments {
            PathArguments::AngleBracketed(args) => Self::analyze_generic(path, last, args),
            _ => Self::analyze_non_generic(path),
        }
    }

    fn analyze_generic(
        path: &syn::Path,
        segment: &syn::PathSegment,
        args: &syn::AngleBracketedGenericArguments,
    ) -> Result<ReturnType, syn::Error> {
        let inner = || -> Result<Option<ReturnType>, syn::Error> {
            for a in &args.args {
                if let GenericArgument::Type(t) = a {
                    return Ok(Some(Self::analyze_type(t)?));
                }
            }
            Ok(None)
        };

        match segment.ident.to_string().as_str() {
            "Vec" => match inner()? {
                Some(t) => Ok(ReturnType::Vec {
                    element_type: Box::new(t),
                }),
                None => Ok(ReturnType::Unknown {
                    name: "Vec<?>".to_string(),
                }),
            },

            "Option" => match inner()? {
                Some(t) => Ok(ReturnType::Option {
                    inner_type: Box::new(t),
                }),
                None => Ok(ReturnType::Unknown {
                    name: "Option<?>".to_string(),
                }),
            },

            "Result" => {
                let mut types = Vec::new();
                for a in &args.args {
                    if let GenericArgument::Type(t) = a {
                        types.push(Self::analyze_type(t)?);
                    }
                }

                match types.len() {
                    2 => {
                        let mut iter = types.into_iter();
                        let ok_type = iter.next().ok_or_else(|| {
                            syn::Error::new(
                                proc_macro2::Span::call_site(),
                                "Result type must have at least one generic parameter",
                            )
                        })?;
                        let err_type = iter.next().ok_or_else(|| {
                            syn::Error::new(
                                proc_macro2::Span::call_site(),
                                "Result type with 2 parameters must have error type",
                            )
                        })?;
                        Ok(ReturnType::Result {
                            ok_type: Box::new(ok_type),
                            err_type: Box::new(err_type),
                        })
                    }
                    1 => {
                        let ok_type = types.into_iter().next().ok_or_else(|| {
                            syn::Error::new(
                                proc_macro2::Span::call_site(),
                                "Result type must have at least one generic parameter",
                            )
                        })?;
                        Ok(ReturnType::Result {
                            ok_type: Box::new(ok_type),
                            err_type: Box::new(ReturnType::Unknown {
                                name: "sqlx_data::Error".to_string(),
                            }),
                        })
                    }
                    _ => Ok(ReturnType::Unknown {
                        name: Self::path_to_string(path),
                    }),
                }
            }

            "Page" => Ok(ReturnType::Unknown {
                name: "Page_deprecated".to_string(),
            }),

            "HashMap" | "BTreeMap" | "IndexMap" | "HashSet" => {
                // Treat HashMap and similar as Struct so they use query_as! instead of query!
                Ok(ReturnType::Struct {
                    name: syn::Ident::new(&segment.ident.to_string(), segment.ident.span()),
                })
            }

            name if pagination::ALL_TYPES.contains(&name) => {
                // Treat pagination types as Struct so they use query_as! and can be detected by pagination logic
                Ok(ReturnType::Struct {
                    name: syn::Ident::new(&segment.ident.to_string(), segment.ident.span()),
                })
            }

            "DateTime" => {
                // DateTime<T> should be treated as a scalar type (DateTime<Utc>, DateTime<Local>, etc.)
                Ok(ReturnType::Scalar {
                    name: syn::Ident::new(&segment.ident.to_string(), segment.ident.span()),
                })
            }

            "Json" => {
                // Json<T> should be treated as a scalar type
                Ok(ReturnType::Scalar {
                    name: syn::Ident::new(&segment.ident.to_string(), segment.ident.span()),
                })
            }

            // PostgreSQL-specific types
            #[cfg(feature = "postgres")]
            "PgRange" | "PgInterval" | "PgMoney" | "PgLTree" | "PgLQuery" | "PgCiText"
            | "PgCube" | "PgPoint" | "PgLine" | "PgLSeg" | "PgBox" | "PgPath"
            | "PgPolygon" | "PgCircle" | "PgHstore" | "PgTimeTz" => {
                Ok(ReturnType::Scalar {
                    name: syn::Ident::new(&segment.ident.to_string(), segment.ident.span()),
                })
            }

            _ => Ok(ReturnType::Unknown {
                name: Self::path_to_string(path),
            }),
        }
    }

    fn analyze_non_generic(path: &syn::Path) -> Result<ReturnType, syn::Error> {
        if Self::is_scalar(path)? {
            Ok(ReturnType::Scalar {
                name: Self::last_ident(path)?.clone(),
            })
        } else {
            Ok(ReturnType::Struct {
                name: Self::last_ident(path)?.clone(),
            })
        }
    }

    fn analyze_tuple(tuple: &syn::TypeTuple) -> ReturnType {
        // Empty tuple is unit type ()
        if tuple.elems.is_empty() {
            return ReturnType::Unit;
        }

        let mut elements = Vec::new();
        for elem in &tuple.elems {
            match Self::analyze_type(elem) {
                Ok(t) => elements.push(t),
                Err(_) => elements.push(ReturnType::Unknown {
                    name: "tuple_element_error".to_string(),
                }),
            }
        }
        ReturnType::Tuple { elements }
    }

    /// Analyze impl Trait types, particularly impl Stream<Item = T>
    fn analyze_impl_trait(impl_trait: &syn::TypeImplTrait) -> Result<ReturnType, syn::Error> {
        // Look for Stream trait bounds
        for bound in &impl_trait.bounds {
            let syn::TypeParamBound::Trait(trait_bound) = bound else {
                continue;
            };

            // Early return if not a Stream trait
            if !Self::path_ends_with(&trait_bound.path, "Stream") {
                continue;
            }

            // Extract Item type from Stream<Item = T>
            if let Some(item_type) = Self::extract_stream_item_type(&trait_bound.path) {
                return Ok(ReturnType::Stream {
                    item_type: Box::new(Self::analyze_type(&item_type)?),
                });
            }
        }

        // Default to unknown for non-Stream impl traits
        Ok(ReturnType::Unknown {
            name: "ImplTrait".to_string(),
        })
    }

    /// Extract the Item type from Stream<Item = T> associated type constraint
    fn extract_stream_item_type(path: &syn::Path) -> Option<syn::Type> {
        let last_segment = path.segments.last()?;
        let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments else {
            return None;
        };

        // Look for Item = T binding in generic arguments
        for arg in &args.args {
            let syn::GenericArgument::AssocType(assoc_type) = arg else {
                continue;
            };

            if assoc_type.ident == "Item" {
                return Some(assoc_type.ty.clone());
            }
        }

        None
    }

    pub fn is_scalar(path: &syn::Path) -> Result<bool, syn::Error> {
        // Check basic scalar types
        let is_basic = Self::is_basic_scalar_type(path);

        if is_basic {
            return Ok(true);
        }

        Ok(Self::is_complex_scalar_type(path))
    }

    fn is_basic_scalar_type(path: &syn::Path) -> bool {
        Self::ends_with_any(
            path,
            &[
                "i8", "i16", "i32", "i64", "i128", "u8", "u16", "u32", "u64", "u128", "f32", "f64",
                "bool", "char", "usize", "isize", "String",
            ],
        )
    }

    fn is_complex_scalar_type(path: &syn::Path) -> bool {
        let is_common = Self::ends_with_any(
            path,
            &[
                // Existing types
                "Cow",
                "Uuid",
                "Hyphenated",
                "Simple",
                "Decimal",
                "BigDecimal",
                "Decimal",
                "Time",
                "Date",
                "DateTime",
                "NaiveDateTime",
                "NaiveDate",
                "NaiveTime",
                "PrimitiveDateTime",
                "OffsetDateTime",
                "MacAddress",
                "Blob",
                "IpNet",
                "Ipv4Net",
                "Ipv6Net",
                "IpNetwork",
                "BitVec",
                "BString",
                "JsonValue",
            ],
        );

        if is_common {
            return true;
        }

        #[cfg(feature = "postgres")]
        if Self::is_postgres_scalar_type(path) {
            return true;
        }

        false
    }

    #[cfg(feature = "postgres")]
    fn is_postgres_scalar_type(path: &syn::Path) -> bool {
        Self::ends_with_any(
            path,
            &[
                "PgInterval",
                "PgMoney",
                "PgLTree",
                "PgLQuery",
                "PgCiText",
                "PgCube",
                "PgPoint",
                "PgLine",
                "PgLSeg",
                "PgBox",
                "PgPath",
                "PgPolygon",
                "PgCircle",
                "PgHstore",
                "PgTimeTz",
            ],
        )
    }

    /// Determine the optimal query strategy for a given type
    pub fn determine_query_strategy(rust_type: &ReturnType) -> syn::Result<QueryType> {
        match rust_type {
            ReturnType::Result { ok_type, .. } => Self::determine_query_strategy(ok_type),

            ReturnType::Unit => Ok(QueryType::Query),

            ReturnType::Scalar { .. } => Ok(QueryType::QueryScalar),

            ReturnType::Struct { .. } => Ok(QueryType::QueryAs),

            ReturnType::Tuple { .. } => Ok(QueryType::QueryAs),

            ReturnType::Vec { element_type } => Self::determine_query_strategy(element_type),

            ReturnType::Option { inner_type } => Self::determine_query_strategy(inner_type),

            // Stream types delegate to their item type strategy
            ReturnType::Stream { item_type } => Self::determine_query_strategy(item_type),

            _ => Ok(QueryType::Query),
        }
    }

    /// Determine fetch method based on return type and context
    pub fn determine_fetch_method(rust_type: &ReturnType) -> FetchMethod {
        match rust_type {
            ReturnType::Result { ok_type, .. } => Self::determine_fetch_method(ok_type),

            ReturnType::Unit => FetchMethod::Execute,

            // Pagination types are treated as structs that return FetchAll
            ReturnType::Struct { name }
                if pagination::ALL_TYPES.contains(&name.to_string().as_str()) =>
            {
                FetchMethod::FetchAll
            }

            ReturnType::Struct { name } if name.to_string().as_str().ends_with("QueryResult") => {
                FetchMethod::Execute
            }

            ReturnType::Struct { name: _ } => FetchMethod::FetchOne,

            ReturnType::Vec { .. } => FetchMethod::FetchAll,

            ReturnType::Option { .. } => FetchMethod::FetchOptional,

            // Stream types use fetch() method for streaming results
            ReturnType::Stream { .. } => FetchMethod::Fetch,

            _ => FetchMethod::FetchOne,
        }
    }

    /// Determine pool expression based on method
    pub fn determine_pool_expr(method: &crate::dml::DmlMethod) -> proc_macro2::TokenStream {
        generate_pool_expr(method)
    }

    /// Determine fetch call expression based on fetch method and pool
    pub fn determine_fetch_call(
        fetch_method: &FetchMethod,
        pool_expr: &proc_macro2::TokenStream,
    ) -> proc_macro2::TokenStream {
        generate_fetch_call_expr(fetch_method, pool_expr)
    }

    /// Check if type is Bytes (bytes::Bytes)
    pub fn is_bytes_type(param_type: &syn::Type) -> bool {
        let syn::Type::Path(type_path) = param_type else {
            return false;
        };
        Self::path_ends_with(&type_path.path, "Bytes")
    }

    /// Returns the inner type of Vec<T> if the type is a Vec.
    /// Example: Vec<String> → Some(&Type::Path("String"))
    ///          String → None
    pub fn get_vec_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
        let syn::Type::Path(syn::TypePath { path, .. }) = ty else {
            return None;
        };

        let segment = path.segments.last()?;

        if segment.ident != "Vec" {
            return None;
        }

        let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
            return None;
        };

        let Some(syn::GenericArgument::Type(inner)) = args.args.first() else {
            return None;
        };

        Some(inner)
    }

    /// Returns the inner type of any iterable collection type.
    /// Supports: Vec<T>, &[T], &mut [T], [T; N], impl IntoIterator<Item = T>
    /// This is the expanded version for multi-insert operations.
    ///
    /// Examples:
    /// - Vec<String> → Some(&Type::Path("String"))
    /// - &[String] → Some(&Type::Path("String"))
    /// - [String; 5] → Some(&Type::Path("String"))
    /// - impl IntoIterator<Item = String> → Some(&Type::Path("String"))
    pub fn get_iterable_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
        match ty {
            // Handle Vec<T>
            syn::Type::Path(_) => Self::get_vec_inner_type(ty),

            // Handle &[T] or &mut [T] (slice references)
            syn::Type::Reference(type_ref) => {
                let syn::Type::Slice(slice_type) = &*type_ref.elem else {
                    return None;
                };
                Some(&*slice_type.elem)
            }

            // Handle [T; N] (array types)
            syn::Type::Array(array_type) => Some(&*array_type.elem),

            // Handle impl IntoIterator<Item = T>
            syn::Type::ImplTrait(impl_trait) => {
                Self::extract_into_iterator_item_type(&impl_trait.bounds)
            }

            _ => None,
        }
    }

    /// Extract the Item type from IntoIterator<Item = T> trait bound
    fn extract_into_iterator_item_type(
        bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::token::Plus>,
    ) -> Option<&syn::Type> {
        for bound in bounds {
            let syn::TypeParamBound::Trait(trait_bound) = bound else {
                continue;
            };

            // Early return if not IntoIterator trait
            if !Self::path_ends_with(&trait_bound.path, "IntoIterator") {
                continue;
            }

            // Extract the last segment which should contain the generic arguments
            let last_segment = trait_bound.path.segments.last()?;
            let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments else {
                continue;
            };

            // Look for Item = T associated type constraint
            for arg in &args.args {
                let syn::GenericArgument::AssocType(assoc_type) = arg else {
                    continue;
                };

                if assoc_type.ident == "Item" {
                    return Some(&assoc_type.ty);
                }
            }
        }
        None
    }

    /// Detects if a type is a tuple-based iterable for multi-insert
    /// Examples: Vec<(i64, String)>, impl IntoIterator<Item = (i64, String, u8)>, I where I: IntoIterator<Item = (...)>
    pub fn is_tuple_iterable_type(ty: &syn::Type, generics: &syn::Generics) -> bool {
        if let Some(inner_type) = Self::get_iterable_inner_type(ty) {
            return Self::is_tuple_type(inner_type);
        }
        if let syn::Type::Path(path) = ty
            && path.path.segments.len() == 1
        {
            let param_name = &path.path.segments[0].ident.to_string();
            return Self::extract_tuple_from_where_clause(generics, param_name).is_some();
        }
        false
    }

    /// Returns true if the type is a tuple (e.g., (i64, String, u8))
    pub fn is_tuple_type(ty: &syn::Type) -> bool {
        matches!(ty, syn::Type::Tuple(_))
    }

    /// Returns true if the type is exactly Vec<u8>
    pub fn is_vec_u8_type(ty: &syn::Type) -> bool {
        let Some(inner) = Self::get_vec_inner_type(ty) else {
            return false;
        };

        Self::is_path_u8(inner)
    }

    /// Checks if a type is u8 (works with u8, std::u8, ::u8, etc.)
    fn is_path_u8(ty: &syn::Type) -> bool {
        let syn::Type::Path(syn::TypePath { path, .. }) = ty else {
            return false;
        };

        let Some(segment) = path.segments.last() else {
            return false;
        };

        segment.ident == "u8"
    }

    /// Check if type is already wrapped with Json<T>
    pub fn is_already_json_wrapped(param_type: &syn::Type) -> bool {
        let syn::Type::Path(type_path) = param_type else {
            return false;
        };
        Self::path_ends_with(&type_path.path, "Json")
    }

    /// Extract inner type from `impl Into<T>` pattern
    /// Returns Some(T) if the type is `impl Into<T>`, None otherwise
    pub fn extract_impl_into_type(param_type: &syn::Type) -> Option<syn::Type> {
        let syn::Type::ImplTrait(impl_trait) = param_type else {
            return None;
        };

        // Find Into<T> bound in trait bounds
        for bound in &impl_trait.bounds {
            let syn::TypeParamBound::Trait(trait_bound) = bound else {
                continue;
            };

            let is_into_trait = Self::path_ends_with(&trait_bound.path, "Into");
            if !is_into_trait {
                continue;
            }

            let Some(last_segment) = trait_bound.path.segments.last() else {
                continue;
            };

            let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments else {
                continue;
            };

            let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() else {
                continue;
            };

            return Some(inner_type.clone());
        }

        None
    }

    /// Extract tuple type from generic parameter with where clause constraint
    /// Example: for generic I where I: IntoIterator<Item = (i64, String)> returns Some((i64, String))
    pub fn extract_tuple_from_where_clause(
        generics: &syn::Generics,
        param_name: &str,
    ) -> Option<syn::Type> {
        // Look through where clause predicates
        let where_clause = generics.where_clause.as_ref()?;

        for predicate in &where_clause.predicates {
            let syn::WherePredicate::Type(type_predicate) = predicate else {
                continue;
            };

            // Check if this predicate is for our parameter
            let syn::Type::Path(bounded_type) = &type_predicate.bounded_ty else {
                continue;
            };

            let Some(last_segment) = bounded_type.path.segments.last() else {
                continue;
            };

            if last_segment.ident != param_name {
                continue;
            }

            // Look for IntoIterator<Item = Tuple> bound
            for bound in &type_predicate.bounds {
                let syn::TypeParamBound::Trait(trait_bound) = bound else {
                    continue;
                };

                // Check if this is IntoIterator trait
                let is_into_iterator = Self::path_ends_with(&trait_bound.path, "IntoIterator");
                if !is_into_iterator {
                    continue;
                }

                // Extract Item type from IntoIterator<Item = T>
                let Some(last_trait_segment) = trait_bound.path.segments.last() else {
                    continue;
                };

                let syn::PathArguments::AngleBracketed(args) = &last_trait_segment.arguments else {
                    continue;
                };

                // Look for Item = Tuple binding
                for arg in &args.args {
                    let syn::GenericArgument::AssocType(binding) = arg else {
                        continue;
                    };

                    if binding.ident != "Item" {
                        continue;
                    }

                    // Check if the Item type is a tuple or reference to tuple
                    if Self::is_tuple_type(&binding.ty) {
                        return Some(binding.ty.clone());
                    }
                    // Also check if it's a reference to a tuple: &(T1, T2, ...)
                    if let syn::Type::Reference(ref_type) = &binding.ty
                        && Self::is_tuple_type(&ref_type.elem)
                    {
                        return Some(binding.ty.clone());
                    }
                }
            }
        }

        None
    }

    fn path_to_string(path: &syn::Path) -> String {
        path.segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
    }

    fn last_ident(path: &syn::Path) -> Result<&syn::Ident, syn::Error> {
        path.segments
            .last()
            .map(|seg| &seg.ident)
            .ok_or_else(|| syn::Error::new(path.span(), "Path has no segments"))
    }

    /// Get the last ident from path safely
    fn last_ident_safe(path: &syn::Path) -> Option<&syn::Ident> {
        path.segments.last().map(|seg| &seg.ident)
    }

    pub fn path_ends_with(path: &syn::Path, name: &str) -> bool {
        Self::last_ident(path)
            .map(|ident| ident == name)
            .unwrap_or(false)
    }

    pub fn ends_with_any(path: &syn::Path, names: &[&str]) -> bool {
        names.iter().any(|n| TypeAnalyzer::path_ends_with(path, n))
    }

    /// Check if return type is Result<Vec<Option<T>>, E>
    pub fn is_vec_option_type(return_type: &syn::Type) -> bool {
        let analyzed = match TypeAnalyzer::analyze_type(return_type) {
            Ok(t) => t,
            Err(_) => {
                log::warn!(
                    "Failed to analyze return type for Vec<Option<T>> detection: {:?}",
                    return_type
                );
                return false;
            }
        };

        match analyzed {
            crate::type_system::ReturnType::Result { ok_type, .. } => {
                // Check if ok_type is Vec<Option<T>>
                matches!(ok_type.as_ref(),
                    crate::type_system::ReturnType::Vec { element_type }
                    if matches!(element_type.as_ref(), crate::type_system::ReturnType::Option { .. })
                )
            }
            _ => false,
        }
    }

    /// Compare two syn::Type for equality
    pub fn types_equal(a: &syn::Type, b: &syn::Type) -> bool {
        // Simple comparison using string representation
        // This works for most cases but could be improved with proper AST comparison
        quote::ToTokens::to_token_stream(a).to_string()
            == quote::ToTokens::to_token_stream(b).to_string()
    }
}

pub struct TypeCastingAnalyzer;
impl TypeCastingAnalyzer {
    pub fn needs_casting(target_type: &syn::Type) -> bool {
        // Check if target type differs from native database type
        let native_type = Self::native_type(target_type);
        !TypeAnalyzer::types_equal(target_type, &native_type)
    }


    /// Check if a type is Option<T> and return the inner type
    pub fn extract_option_type(ty: &syn::Type) -> Option<syn::Type> {
        let syn::Type::Path(syn::TypePath { path, .. }) = ty else {
            return None;
        };

        if !TypeAnalyzer::path_ends_with(path, "Option") {
            return None;
        }

        let last_segment = path.segments.last()?;
        let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments else {
            return None;
        };

        let syn::GenericArgument::Type(inner_type) = args.args.first()? else {
            return None;
        };

        Some(inner_type.clone())
    }

    pub fn native_type(ty: &syn::Type) -> syn::Type {
        match ty {
            syn::Type::Path(syn::TypePath { path, .. }) => {
                let Some(ident) = TypeAnalyzer::last_ident_safe(path) else {
                    return ty.clone();
                };

                match ident {
                    #[cfg(feature = "sqlite")]
                    ident
                    if ident == "u64"
                    || ident == "u32"
                    || ident == "u16"
                    || ident == "u8"
                    || ident == "usize"
                    || ident == "i32"
                    || ident == "i16"
                    || ident == "i8"
                    || ident == "isize" =>
                    {
                        syn::parse_quote!(i64)
                    }
                    #[cfg(feature = "sqlite")]
                    ident if ident == "f32" => syn::parse_quote!(f64),
                    // SQLite compatibility + universal read safety
                    #[cfg(feature = "sqlite")]
                    ident if ident == "bool" => syn::parse_quote!(i64),
                    _ => ty.clone(),
                }
            }
            // Non-path types (reference, tuple, etc.)
            _ => ty.clone(),
        }
    }



}

/// Simple extract: just the column name part from SQLx cast syntax
/// Examples: "email_length: i64" -> "email_length", "email_length!: i32" -> "email_length", "name" -> "name"
pub fn extract_column_name(full_name: &str) -> &str {
    let base_name = if let Some(colon_pos) = full_name.find(':') {
        full_name[..colon_pos].trim()
    } else {
        full_name
    };

    // Remove trailing ! from SQLx force cast syntax
    base_name.trim_end_matches('!')
}

/// Extract explicit SQLx type from column name if present
/// Returns the Rust type specified in SQLx cast syntax like 'name: String' or 'age!: u8'
pub fn extract_sqlx_explicit_type(column_name: &str) -> Option<syn::Type> {
    let colon_pos = column_name.find(':')?;
    let type_part = column_name[colon_pos + 1..].trim();

    // Parse the type string into a syn::Type
    syn::parse_str(type_part).ok()
}

/// Check if a column has explicit SQLx casting syntax
/// Uses robust regex matching to avoid false positives with time values, URLs, etc.
pub fn has_explicit_sqlx_type(column_name: &str) -> bool {
    // For individual column names (from inferred columns), simple contains is sufficient
    // since column names are already extracted and don't contain SQL literals
    column_name.contains(':')
}

/// Clean SQLx cast syntax from SQL for runtime execution
/// Removes patterns like 'column: Type' or "column: Type" or `column: Type`
/// while preserving normal aliases like 'column_alias'
pub fn clean_sqlx_cast_syntax_for_runtime(sql: &str) -> String {
    use crate::constants::regex::SQLX_CAST_CLEANER;

    // Replace with just the column name (remove all SQLx modifiers and type info)
    SQLX_CAST_CLEANER.replace_all(sql, "$2").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_scalar_type_analysis() {
        let ty: Type = parse_quote!(i64);
        let analyzed = TypeAnalyzer::analyze_type(&ty).unwrap();

        match analyzed {
            ReturnType::Scalar { name } => {
                assert_eq!(name, "i64");
            }
            _ => panic!("Expected scalar type"),
        }
    }

    #[test]
    fn test_extract_sqlx_explicit_type() {
        // Test valid explicit types
        assert!(extract_sqlx_explicit_type("name: String").is_some());
        assert!(extract_sqlx_explicit_type("age: u8").is_some());
        assert!(extract_sqlx_explicit_type("count!: i64").is_some());
        assert!(extract_sqlx_explicit_type("name : String ").is_some()); // with spaces

        // Test no explicit type
        assert!(extract_sqlx_explicit_type("name").is_none());
        assert!(extract_sqlx_explicit_type("age_value").is_none());
    }

    #[test]
    fn test_has_explicit_sqlx_type() {
        assert!(has_explicit_sqlx_type("name: String"));
        assert!(has_explicit_sqlx_type("age!: u8"));
        assert!(has_explicit_sqlx_type("count : i64"));

        assert!(!has_explicit_sqlx_type("name"));
        assert!(!has_explicit_sqlx_type("age_value"));
        assert!(!has_explicit_sqlx_type("user_count"));
    }

    #[test]
    fn test_option_type_analysis() {
        let ty: Type = parse_quote!(Option<String>);
        let analyzed = TypeAnalyzer::analyze_type(&ty).unwrap();

        match analyzed {
            ReturnType::Option { inner_type } => match inner_type.as_ref() {
                ReturnType::Scalar { name, .. } => assert_eq!(name, "String"),
                _ => panic!("Expected inner scalar type"),
            },
            _ => panic!("Expected Option type"),
        }
    }

    #[test]
    fn test_vec_type_analysis() {
        let ty: Type = parse_quote!(Vec<User>);
        let analyzed = TypeAnalyzer::analyze_type(&ty).unwrap();

        match analyzed {
            ReturnType::Vec { element_type } => match element_type.as_ref() {
                ReturnType::Struct { name } => assert_eq!(name, "User"),
                _ => panic!("Expected inner struct type"),
            },
            _ => panic!("Expected Vec type"),
        }
    }

    #[test]
    fn test_query_strategy_determination() {
        use syn::parse_quote;

        let ty: Type = parse_quote!(Result<Vec<User>>);
        let analyzed = TypeAnalyzer::analyze_type(&ty).unwrap();

        let query_type = TypeAnalyzer::determine_query_strategy(&analyzed);

        match query_type {
            Ok(QueryType::QueryAs) => {
                // Test passes - correct strategy determined
            }
            _ => panic!("Expected QueryAs strategy"),
        }
    }

    #[test]
    fn test_is_scalar_comprehensive() {
        use syn::parse_quote;

        // Basic scalar types - should return true
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(i8)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(i16)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(i32)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(i64)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(i128)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(u8)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(u16)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(u32)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(u64)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(u128)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(f32)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(f64)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(bool)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(char)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(usize)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(isize)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(String)).unwrap());

        // Complex scalar types - should return true
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(NaiveDateTime)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(NaiveDate)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(NaiveTime)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(DateTime<Utc>)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(Uuid)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(Decimal)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(Blob)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(Cow<str>)).unwrap());

        // Fully qualified paths - should return true
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(std::string::String)).unwrap());
        assert!(
            TypeAnalyzer::is_scalar(&parse_quote!(sqlx::types::chrono::NaiveDateTime)).unwrap()
        );
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(chrono::NaiveDate)).unwrap());
        assert!(TypeAnalyzer::is_scalar(&parse_quote!(::std::primitive::i64)).unwrap());

        // Non-scalar types - should return false
        assert!(!TypeAnalyzer::is_scalar(&parse_quote!(Vec<i32>)).unwrap());
        assert!(!TypeAnalyzer::is_scalar(&parse_quote!(Option<String>)).unwrap());
        assert!(!TypeAnalyzer::is_scalar(&parse_quote!(Result<i64, String>)).unwrap());
        assert!(!TypeAnalyzer::is_scalar(&parse_quote!(HashMap<String, i64>)).unwrap());
        // Tuples, arrays, references can cause parse issues - skipping these checks
        assert!(!TypeAnalyzer::is_scalar(&parse_quote!(User)).unwrap());
        assert!(!TypeAnalyzer::is_scalar(&parse_quote!(CustomStruct)).unwrap());

        // Edge cases that should fail
        assert!(!TypeAnalyzer::is_scalar(&parse_quote!(Box<i64>)).unwrap());
        assert!(!TypeAnalyzer::is_scalar(&parse_quote!(Arc<String>)).unwrap());
        assert!(!TypeAnalyzer::is_scalar(&parse_quote!(Rc<NaiveDateTime>)).unwrap());
    }
}
