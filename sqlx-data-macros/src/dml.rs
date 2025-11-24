use crate::alias_system::AliasManager;
use crate::constants::pagination;
use crate::constants::regex::NAMED_PARAM_REGEX;
use crate::error::core_error;
use crate::scope_system::ScopeManager;
use crate::type_system::SqlSource;
use sqlparser::ast::Statement;
use sqlx_data_parser::PLACEHOLDER;
use sqlx_data_parser::SqlStatementType;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use syn::spanned::Spanned;
use syn::{FnArg, Pat, PatType, TraitItemFn, Type};

/// Cached return type analysis to avoid multiple AST traversals
#[derive(Debug, Clone)]
pub struct ReturnTypeInfo {
    pub inner: Type,
    pub is_tuple: bool,
    pub is_stream: bool,
    pub ok_type: Option<Type>, // Used by return_ok_type_name() indirectly
    pub ok_type_from_inner: Option<Type>, // Extract from inner type - may be needed
    pub ok_type_name: Option<String>,
    pub is_pagination: bool,
}

impl ReturnTypeInfo {
    fn analyze(return_type: Option<&Type>) -> Self {
        let Some(ty) = return_type else {
            return Self::default_unit();
        };

        let inner = Self::extract_inner_type(ty);
        let is_stream = Self::check_is_stream_type(ty);
        let ok_type = Self::extract_first_type_arg(ty); // Extract from original type
        let ok_type_from_inner = Self::extract_ok_type_from_inner(&inner); // Extract from inner type
        let ok_type_name = Self::extract_ok_type_name(&ok_type);
        let is_pagination = Self::check_is_pagination(&ok_type_name);
        let is_tuple = Self::check_is_tuple(&inner, &ok_type_from_inner);

        Self {
            inner,
            is_tuple,
            is_stream,
            ok_type,
            ok_type_from_inner,
            ok_type_name,
            is_pagination,
        }
    }

    /// Extract first type argument from generic type: Result<T> → T, Vec<T> → T, etc.
    /// This is the core logic that was in get_first_type_arg
    fn extract_first_type_arg(ty: &Type) -> Option<Type> {
        match ty {
            syn::Type::Path(path) => {
                let segment = path.path.segments.last()?;
                let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
                    return None;
                };
                let syn::GenericArgument::Type(inner) = args.args.first()? else {
                    return None;
                };
                Some(inner.clone())
            }
            syn::Type::ImplTrait(impl_trait) => {
                // Look for trait bounds with associated types
                for bound in &impl_trait.bounds {
                    let syn::TypeParamBound::Trait(trait_bound) = bound else {
                        continue;
                    };
                    let Some(last_segment) = trait_bound.path.segments.last() else {
                        continue;
                    };
                    let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments else {
                        continue;
                    };

                    // Look for Item = T associated type (Stream<Item = T>)
                    for arg in &args.args {
                        if let syn::GenericArgument::AssocType(assoc_type) = arg
                            && assoc_type.ident == "Item"
                        {
                            return Some(assoc_type.ty.clone());
                        }
                    }

                    // Fallback: get first generic type argument
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner.clone());
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Extract ok_type from inner type: Result<User> → User
    fn extract_ok_type_from_inner(inner_ty: &Type) -> Option<Type> {
        Self::extract_first_type_arg(inner_ty)
    }

    fn default_unit() -> Self {
        Self {
            inner: syn::parse_quote! { () },
            is_tuple: true,
            is_stream: false,
            ok_type: None,
            ok_type_from_inner: None,
            ok_type_name: None,
            is_pagination: false,
        }
    }

    fn extract_inner_type(ty: &Type) -> Type {
        let mut current = ty.clone();

        loop {
            match &current {
                syn::Type::Path(path) => {
                    let Some(segment) = path.path.segments.last() else {
                        break;
                    };
                    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
                        break;
                    };
                    let Some(syn::GenericArgument::Type(inner)) = args.args.first() else {
                        break;
                    };
                    current = inner.clone();
                }
                syn::Type::ImplTrait(impl_trait) => {
                    // Look for Stream<Item = T> pattern
                    for bound in &impl_trait.bounds {
                        let syn::TypeParamBound::Trait(trait_bound) = bound else {
                            continue;
                        };
                        let Some(last_segment) = trait_bound.path.segments.last() else {
                            continue;
                        };
                        let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments
                        else {
                            continue;
                        };

                        // Look for Item = T associated type
                        for arg in &args.args {
                            if let syn::GenericArgument::AssocType(assoc_type) = arg
                                && assoc_type.ident == "Item"
                            {
                                return assoc_type.ty.clone();
                            }
                        }

                        // Fallback: get first generic type argument
                        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                            return inner.clone();
                        }
                    }
                    break;
                }
                _ => break,
            }
        }

        current
    }

    fn extract_ok_type_name(ok_type: &Option<Type>) -> Option<String> {
        let Some(syn::Type::Path(path)) = ok_type else {
            return None;
        };
        let segment = path.path.segments.last()?;
        Some(segment.ident.to_string())
    }

    fn check_is_stream_type(ty: &Type) -> bool {
        let syn::Type::ImplTrait(impl_trait) = ty else {
            return false;
        };
        impl_trait.bounds.iter().any(|bound| {
            matches!(bound, syn::TypeParamBound::Trait(trait_bound)
                if crate::type_analyzer::TypeAnalyzer::path_ends_with(
                    &trait_bound.path,
                    "Stream"
                )
            )
        })
    }

    fn check_is_pagination(ok_type_name: &Option<String>) -> bool {
        ok_type_name
            .as_ref()
            .is_some_and(|name| pagination::ALL_TYPES.contains(&name.as_str()))
    }

    fn check_is_tuple(inner: &Type, ok_type_from_inner: &Option<Type>) -> bool {
        // For streams and complex types, check ok_type_from_inner first
        ok_type_from_inner
            .as_ref()
            .map(|t| matches!(t, syn::Type::Tuple(_)))
            .unwrap_or_else(|| matches!(inner, syn::Type::Tuple(_)))
    }
}

/// Parsed DML method information
#[derive(Clone)]
pub struct DmlMethod {
    pub method: TraitItemFn,           // Complete method AST
    pub sql_content: String,           // Final resolved SQL content
    pub parameters: Vec<DmlParameter>, // All parameters (query + pool + Params)
    #[allow(dead_code)]
    pub statement: Option<Arc<Statement>>,
    pub kind: SqlStatementType,
    pub is_json_query: bool,   // True if json flag is specified in #[dml]
    pub is_multi_insert: bool, // True if method has Vec parameters for multi-row insert
    pub is_unchecked: bool,    // True if unchecked flag is specified in #[dml]
    pub has_explicit_instrument: bool, // True if user provided #[instrument] attribute
    pub trait_instrument: bool, // True if trait has instrument = true in #[repo]

    // Cached return type analysis - computed once, used many times
    pub return_info_cache: OnceLock<ReturnTypeInfo>,
}

#[derive(Clone)]
pub struct DmlParameter {
    pub name: String,
    pub type_: Type,
    pub is_pool: bool,          // True if this is a SQLx Pool parameter
    pub is_dynamic_param: bool, // Dynamic parameter type for query building
    pub is_generic: bool,       // True if this is impl trait or generic type parameter
}

impl DmlMethod {
    /// Get method name
    pub fn name(&self) -> String {
        self.method.sig.ident.to_string()
    }

    /// Get cached return type info (computed once, reused many times)
    fn return_info(&self) -> &ReturnTypeInfo {
        self.return_info_cache.get_or_init(|| {
            let return_type = match &self.method.sig.output {
                syn::ReturnType::Type(_, ty) => Some(ty.as_ref()),
                syn::ReturnType::Default => None,
            };
            ReturnTypeInfo::analyze(return_type)
        })
    }

    /// Get method return type, returns Result<()> for default
    pub fn return_type(&self) -> Option<&Type> {
        match &self.method.sig.output {
            syn::ReturnType::Type(_, ty) => Some(ty),
            syn::ReturnType::Default => None,
        }
    }

    /// Get method span
    pub fn method_span(&self) -> proc_macro2::Span {
        self.method.span()
    }

    /// Get method generics
    pub fn generics(&self) -> &syn::Generics {
        &self.method.sig.generics
    }

    /// Check if method is async
    pub fn is_async(&self) -> bool {
        self.method.sig.asyncness.is_some()
    }

    /// Get inner type from return type: Result<Serial<User>> → User
    /// For Vec<Option<u64>> → u64
    pub fn get_return_inner_type(&self) -> &Type {
        &self.return_info().inner
    }

    /// Get ok type from inner type: Result<User> → User
    pub fn get_ok_type(&self) -> Option<&Type> {
        // For streams, we need to extract from inner type (Result<User> → User)
        if self.is_stream_type() {
            self.return_info().ok_type_from_inner.as_ref()
        } else {
            self.return_info().ok_type.as_ref()
        }
    }

    /// Check if this is a data modification statement that may need Json wrapping
    pub fn is_data_modification(&self) -> bool {
        matches!(
            self.kind,
            SqlStatementType::Insert | SqlStatementType::Update | SqlStatementType::Delete
        )
    }

    /// Check if this method has CRUD operation
    pub fn is_crud_operation(&self) -> bool {
        matches!(
            self.kind,
            SqlStatementType::Select
                | SqlStatementType::Insert
                | SqlStatementType::Update
                | SqlStatementType::Delete
        )
    }

    /// Check if this method has any dynamic parameter types
    #[allow(dead_code)]
    pub fn has_dynamic_params(&self) -> bool {
        self.parameters.iter().any(|p| p.is_dynamic_param)
    }

    /// Get the type name (ident) from the ok_type
    /// For Result<Serial<User>, Error> → "Serial"
    /// For Vec<User> → "Vec"
    /// For String → "String"
    pub fn return_ok_type_name(&self) -> Option<String> {
        self.return_info().ok_type_name.clone()
    }

    /// Check if the return type is a pagination type (Serial, Slice, or Cursor)
    pub fn is_pagination_type(&self) -> bool {
        self.return_info().is_pagination
    }

    /// Check if the return type is a Stream type (impl Stream<Item = T>)
    pub fn is_stream_type(&self) -> bool {
        self.return_info().is_stream
    }

    /// Check if this method uses multi-insert (has Vec parameters)
    pub fn is_multi_insert(&self) -> bool {
        self.is_multi_insert
    }

    /// Check if the inner return type is a tuple
    pub fn is_tuple_type(&self) -> bool {
        self.return_info().is_tuple
    }

    /// Parse tuple types from the inner return type
    pub fn parse_tuple_types(&self) -> syn::Result<Vec<syn::Type>> {
        let info = self.return_info();

        // For streams/complex types, use ok_type_from_inner; otherwise use inner
        let tuple_type = if let Some(ok_from_inner) = &info.ok_type_from_inner {
            ok_from_inner
        } else {
            &info.inner
        };

        let syn::Type::Tuple(tuple) = tuple_type else {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "Expected tuple type",
            ));
        };

        Ok(tuple.elems.iter().cloned().collect())
    }
}

pub struct DmlParser;

impl DmlParser {
    /// Parse a trait method with direct args from proc macro
    pub fn parse_dml_method_with_args(
        method: TraitItemFn,
        args: proc_macro::TokenStream,
        trait_instrument: bool,
    ) -> syn::Result<DmlMethod> {
        // Extract return type
        let return_type = match &method.sig.output {
            syn::ReturnType::Type(_, ty) => ty.as_ref().clone(),
            syn::ReturnType::Default => syn::parse_quote! { Result<()> },
        };

        // Extract aliases from method attributes (injected by #[repo])
        let alias_manager = AliasManager::extract_from_method_attributes(&method.attrs)?;

        // Extract scopes from method attributes (injected by #[repo])
        let scope_manager = ScopeManager::extract_from_method_attributes(&method.attrs)?;

        // Check if user explicitly provided #[instrument] attribute
        let has_explicit_instrument = Self::has_explicit_instrument_attr(&method.attrs);

        // Extract trait-level instrument flag from injected attributes
        let trait_instrument_from_attrs = Self::extract_trait_instrument_flag(&method.attrs);

        // Extract parameters first to use in named parameter conversion
        let parameters = Self::extract_parameters(&method.sig.inputs, &method.sig.generics)?;

        // Parse the macro arguments directly
        let (sql_source, has_unchecked, has_json, has_multi_insert) = Self::parse_macro_args(args)?;

        // Pre-parsing validations (don't need SQL analysis)
        Self::run_pre_parsing_validations(&return_type, &method, has_unchecked)?;

        // Clean linear pipeline - resolve file only once, zero-cost with Cow
        let sql_content = sql_source.resolve_content()?; // File → String (resolve once)
        let sql_content = Self::apply_aliases_if_needed(&sql_content, &alias_manager)?; // Aliases
        let sql_content = Self::apply_scopes_if_needed(sql_content, &scope_manager)?; // Scopes
        let sql_content = Self::convert_named_to_positional_if_needed(sql_content, &parameters)?; // Named params

        let statement = sqlx_data_parser::parse_sql(&sql_content).map_err(core_error)?;
        let kind = Self::detect_sql_type(statement.as_ref());

        // Post-parsing validations (need SQL analysis results)
        Self::run_post_parsing_validations(&return_type, &method, &kind, has_unchecked)?;

        // Auto-detect multi-insert if not explicitly set
        let final_multi_insert = has_multi_insert
            || Self::is_auto_multi_insert(&kind, &parameters, &method.sig.generics);

        Ok(DmlMethod {
            method,
            sql_content,
            parameters,
            has_explicit_instrument,
            trait_instrument: trait_instrument || trait_instrument_from_attrs,
            is_json_query: has_json,
            is_multi_insert: final_multi_insert,
            is_unchecked: has_unchecked,
            statement,
            kind,
            return_info_cache: OnceLock::new(),
        })
    }

    /// Check if method has explicit #[instrument] attribute
    fn has_explicit_instrument_attr(attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|attr| attr.path().is_ident("instrument"))
    }

    /// Extract trait-level instrument flag from injected attributes
    fn extract_trait_instrument_flag(attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|attr| {
            attr.path().is_ident("sqlx_data_trait_instrument")
                && attr.meta.require_name_value().is_ok_and(|nv| {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Bool(lit_bool),
                        ..
                    }) = &nv.value
                    {
                        lit_bool.value
                    } else {
                        false
                    }
                })
        })
    }

    /// Parse macro arguments using proper syn parsing
    fn parse_macro_args(
        args: proc_macro::TokenStream,
    ) -> syn::Result<(SqlSource, bool, bool, bool)> {
        use syn::{Expr, Token, parse::Parser, punctuated::Punctuated};

        if args.is_empty() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "Attribute requires either a SQL string or file parameter",
            ));
        }

        let parser = Punctuated::<Expr, Token![,]>::parse_terminated;
        let args = parser.parse(args)?;

        if args.is_empty() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "Attribute requires at least one argument",
            ));
        }

        let mut sql_source: Option<SqlSource> = None;
        let mut has_unchecked = false;
        let mut has_json = false;
        let mut has_multi_insert = false;

        for arg in &args {
            match arg {
                // --- Inline SQL: #[dml("SELECT ...")]
                Expr::Lit(expr_lit) if sql_source.is_none() => {
                    let syn::Lit::Str(lit_str) = &expr_lit.lit else {
                        return Err(syn::Error::new_spanned(
                            &expr_lit.lit,
                            "SQL argument must be a string literal",
                        ));
                    };

                    sql_source = Some(SqlSource::Inline(lit_str.value()));
                }

                // --- Key/value: file="path"
                Expr::Assign(assign) => {
                    let Expr::Path(path) = &*assign.left else {
                        return Err(syn::Error::new_spanned(
                            &assign.left,
                            "Invalid parameter name",
                        ));
                    };

                    if !path.path.is_ident("file") {
                        return Err(syn::Error::new_spanned(
                            path,
                            "Unknown parameter. Supported: file",
                        ));
                    }

                    if sql_source.is_some() {
                        return Err(syn::Error::new_spanned(
                            assign,
                            "Cannot specify both inline SQL and file parameter",
                        ));
                    }

                    let Expr::Lit(expr_lit) = &*assign.right else {
                        return Err(syn::Error::new_spanned(
                            &assign.right,
                            "file parameter must be a string literal",
                        ));
                    };

                    let syn::Lit::Str(lit_str) = &expr_lit.lit else {
                        return Err(syn::Error::new_spanned(
                            &assign.right,
                            "file parameter must be a string literal",
                        ));
                    };

                    sql_source = Some(SqlSource::File(lit_str.value()));
                }

                // --- unchecked flag: #[dml("SQL", unchecked)]
                Expr::Path(path) if path.path.is_ident("unchecked") => {
                    has_unchecked = true;
                }

                // --- json flag: #[dml("SQL", json)]
                Expr::Path(path) if path.path.is_ident("json") => {
                    has_json = true;
                }

                // --- multi_insert flag: #[dml("SQL", multi_insert)]
                Expr::Path(path) if path.path.is_ident("multi_insert") => {
                    has_multi_insert = true;
                }

                // --- Anything else: erro
                _ => {
                    return Err(syn::Error::new_spanned(
                        arg,
                        "Arguments must be either a SQL string literal, key=value pairs, 'unchecked' flag, 'json' flag, or 'multi_insert' flag",
                    ));
                }
            }
        }

        let sql = sql_source.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "DML attribute requires either a SQL string or file parameter",
            )
        })?;

        Ok((sql, has_unchecked, has_json, has_multi_insert))
    }

    /// Automatically detect if this should be a multi-insert operation
    fn is_auto_multi_insert(
        kind: &SqlStatementType,
        parameters: &[DmlParameter],
        generics: &syn::Generics,
    ) -> bool {
        // Early return: not an INSERT statement
        if !matches!(kind, SqlStatementType::Insert) {
            return false;
        }

        // Early return: check if any parameter is a Vec of tuples
        parameters
            .iter()
            .filter(|p| !p.is_pool && !p.is_dynamic_param)
            .any(|p| crate::type_analyzer::TypeAnalyzer::is_tuple_iterable_type(&p.type_, generics))
    }

    /// Extract parameters from function signature
    fn extract_parameters(
        inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
        generics: &syn::Generics,
    ) -> syn::Result<Vec<DmlParameter>> {
        let mut parameters = Vec::new();
        let mut has_pool = false;

        for input in inputs {
            match input {
                FnArg::Receiver(_) => {
                    // Skip &self
                    continue;
                }
                FnArg::Typed(PatType { pat, ty, .. }) => {
                    let Pat::Ident(ident) = &**pat else {
                        return Err(syn::Error::new_spanned(
                            pat,
                            "Only simple parameter names are supported",
                        ));
                    };

                    let is_pool = Self::is_sqlx_executor_type(ty, generics);
                    let is_dynamic_param = Self::is_dynamic_param_type(ty);

                    // Check for multiple pool parameters
                    if is_pool {
                        if has_pool {
                            return Err(syn::Error::new_spanned(
                                pat,
                                "Only one pool parameter is allowed per method",
                            ));
                        }
                        has_pool = true;
                    }

                    let is_generic = Self::is_generic_type(ty, generics);

                    parameters.push(DmlParameter {
                        name: ident.ident.to_string(),
                        type_: (**ty).clone(),
                        is_pool,
                        is_dynamic_param,
                        is_generic,
                    });
                }
            }
        }

        Ok(parameters)
    }

    /// Check if a type is a SQLx executor type using proper trait bound analysis
    fn is_sqlx_executor_type(ty: &syn::Type, generics: &syn::Generics) -> bool {
        match ty {
            // Handle impl sqlx::Executor or impl Executor
            syn::Type::ImplTrait(impl_trait) => Self::implements_sqlx_executor(&impl_trait.bounds),
            // Handle regular references like &Pool, &mut Connection, etc.
            syn::Type::Reference(type_ref) => Self::is_concrete_sqlx_executor_type(&type_ref.elem),
            // Handle generic types - check if they have executor bounds
            syn::Type::Path(type_path) => {
                let Some(segment) = type_path.path.segments.last() else {
                    return false;
                };

                let type_name = segment.ident.to_string();
                if Self::is_generic_executor(&type_name, generics) {
                    return true;
                }
                Self::is_concrete_sqlx_executor_type(ty)
            }
            _ => false,
        }
    }

    /// Check if bounds include sqlx::Executor
    fn implements_sqlx_executor(
        bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::token::Plus>,
    ) -> bool {
        bounds.iter().any(|bound| {
            if let syn::TypeParamBound::Trait(trait_bound) = bound {
                let path_segments: Vec<String> = trait_bound
                    .path
                    .segments
                    .iter()
                    .map(|seg| seg.ident.to_string())
                    .collect();

                // Check for "Executor", "sqlx::Executor", or "sqlx_data::Executor"
                path_segments == vec!["Executor"]
                    || path_segments == vec!["sqlx", "Executor"]
                    || path_segments == vec!["sqlx_data", "Executor"]
            } else {
                false
            }
        })
    }

    /// Check if a type is a known concrete SQLx executor type
    fn is_concrete_sqlx_executor_type(ty: &syn::Type) -> bool {
        let syn::Type::Path(type_path) = ty else {
            return false;
        };
        let Some(segment) = type_path.path.segments.last() else {
            return false;
        };

        let segment_name = segment.ident.to_string();

        // Check for Pool (accepts both Pool<T> and Pool without generics)
        if segment.ident == "Pool" {
            return true; // Support both &Pool<sqlx::Sqlite> and &Pool
        }

        // Check for Transaction (requires angle brackets for lifetime and DB parameters)
        if segment.ident == "Transaction" {
            return matches!(&segment.arguments, syn::PathArguments::AngleBracketed(_));
        }

        // Check for Connection types (SqliteConnection, PgConnection, MySqlConnection, Connection)
        segment_name.ends_with("Connection")
    }

    /// Check if a type is a generic type that doesn't support .is_empty()
    fn is_generic_type(ty: &syn::Type, generics: &syn::Generics) -> bool {
        match ty {
            // impl IntoIterator types don't support .is_empty()
            syn::Type::ImplTrait(_) => true,
            // Check if it's a generic parameter
            syn::Type::Path(path) => {
                if path.path.segments.len() == 1 && path.path.segments[0].arguments.is_empty() {
                    let param_name = path.path.segments[0].ident.to_string();
                    // Check if this type name is a generic parameter
                    generics.params.iter().any(|param| {
                        matches!(param, syn::GenericParam::Type(type_param) if type_param.ident == param_name)
                    })
                } else {
                    false // Multi-segment path or with generics - concrete type
                }
            }
            _ => false,
        }
    }

    /// Check if a generic type name has sqlx::Executor bound in inline bounds or where clause
    fn is_generic_executor(type_name: &str, generics: &syn::Generics) -> bool {
        // First check inline bounds on generic parameters like E: Executor<'e>
        for param in &generics.params {
            if let syn::GenericParam::Type(type_param) = param
                && type_param.ident == type_name
            {
                // Only check inline bounds if they exist
                if !type_param.bounds.is_empty() {
                    return Self::implements_sqlx_executor(&type_param.bounds);
                }
                // If no inline bounds, continue to where clause check
                break;
            }
        }

        // Then check where clause bounds
        let Some(where_clause) = &generics.where_clause else {
            return false;
        };

        for predicate in &where_clause.predicates {
            let syn::WherePredicate::Type(type_predicate) = predicate else {
                continue;
            };

            let syn::Type::Path(bounded_type) = &type_predicate.bounded_ty else {
                continue;
            };

            let Some(segment) = bounded_type.path.segments.last() else {
                continue;
            };

            if segment.ident == type_name {
                return Self::implements_sqlx_executor(&type_predicate.bounds);
            }
        }

        false
    }

    /// Check if a type is a dynamic parameter type
    fn is_dynamic_param_type(ty: &syn::Type) -> bool {
        match ty {
            // Handle impl IntoParams
            syn::Type::ImplTrait(impl_trait) => {
                for bound in &impl_trait.bounds {
                    if let syn::TypeParamBound::Trait(trait_bound) = bound
                        && let Some(segment) = trait_bound.path.segments.last()
                        && segment.ident == "IntoParams"
                    {
                        return true;
                    }
                }
                false
            }
            // Handle concrete types: Params, SearchParams, etc.
            syn::Type::Path(type_path) => {
                let Some(segment) = type_path.path.segments.last() else {
                    return false;
                };

                matches!(
                    segment.ident.to_string().as_str(),
                    "Params"
                        | "SearchParams"
                        | "PaginationParams"
                        | "SortingParams"
                        | "FilterParams"
                        | "CursorParams"
                        | "SerialParams"
                        | "SliceParams"
                        | "ParamsBuilder"
                )
            }
            _ => false,
        }
    }

    /// Apply aliases if needed (zero-cost with Cow)
    fn apply_aliases_if_needed<'a>(
        sql: &'a str,
        alias_manager: &AliasManager,
    ) -> syn::Result<Cow<'a, str>> {
        if alias_manager.has_aliases() {
            alias_manager.substitute_aliases(sql).map(Cow::Owned)
        } else {
            Ok(Cow::Borrowed(sql))
        }
    }

    /// Apply scopes if needed (zero-cost with Cow)
    fn apply_scopes_if_needed<'a>(
        sql: Cow<'a, str>,
        scope_manager: &ScopeManager,
    ) -> syn::Result<Cow<'a, str>> {
        if scope_manager.has_active_scopes() {
            scope_manager.apply_scopes_to_sql(&sql).map(Cow::Owned)
        } else {
            Ok(sql)
        }
    }

    /// Convert named parameters if needed (zero-cost with Cow)
    fn convert_named_to_positional_if_needed<'a>(
        sql: Cow<'a, str>,
        parameters: &[DmlParameter],
    ) -> syn::Result<String> {
        Self::convert_named_to_positional_in_sql(&sql, parameters)
    }

    /// Convert named parameters (@name) to positional parameters ($1, $2, etc.)
    fn convert_named_to_positional_in_sql(
        sql: &str,
        parameters: &[DmlParameter],
    ) -> syn::Result<String> {
        // Early return: check if SQL contains any named parameters (@)
        if !sql.contains('@') {
            return Ok(sql.to_string());
        }

        if !NAMED_PARAM_REGEX.is_match(sql).map_err(|e| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("Regex error checking for named parameters: {}", e),
            )
        })? {
            return Ok(sql.to_string());
        }
        let param_map: HashMap<&str, usize> = parameters
            .iter()
            .filter(|p| !p.is_pool && !p.is_dynamic_param)
            .enumerate()
            .map(|(index, param)| (param.name.as_str(), index + 1))
            .collect();

        // Early return: if no valid parameters, just return original SQL
        if param_map.is_empty() {
            return Ok(sql.to_string());
        }

        let mut unknown_params = Vec::new();
        let result = NAMED_PARAM_REGEX.replace_all(sql, |caps: &fancy_regex::Captures| {
            let prefix = &caps[1]; // Captures the text before the parameter (e.g., spaces, parentheses, etc.)
            let param_name = &caps[2]; // Captures the actual parameter name (without the @)

            match param_map.get(param_name) {
                // Case 1: Parameter exists and we're targeting MySQL (uses "?" placeholder)
                Some(_) if PLACEHOLDER() == "?" => {
                    format!("{prefix}{}", PLACEHOLDER())
                }

                // Case 2: Parameter exists and we're targeting Postgres or SQLite (uses numbered placeholders like $1, $2...)
                Some(&index) => {
                    format!("{prefix}{}{index}", PLACEHOLDER())
                }

                // Case 3: Parameter not found in the param_map
                // We collect it to report unknown/missing parameters later
                None => {
                    if !unknown_params.contains(&param_name.to_string()) {
                        unknown_params.push(param_name.to_string());
                    }
                    // Keep the original @param_name in the query so it doesn't break execution
                    // and makes debugging easier
                    format!("{prefix}@{param_name}")
                }
            }
        });

        if !unknown_params.is_empty() {
            let available_params: Vec<&str> = param_map.keys().cloned().collect();
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "Unknown named parameters in SQL: [{}]. Available parameters: [{}]",
                    unknown_params.join(", "),
                    available_params.join(", ")
                ),
            ));
        }

        Ok(result.into_owned())
    }

    /// Check if the return type contains Unit type ()
    fn return_type_contains_unit(return_type: &syn::Type) -> bool {
        // Direct unit type check
        if Self::is_unit_type(return_type) {
            return true;
        }

        // Check for generic types like Result<()>
        let syn::Type::Path(type_path) = return_type else {
            return false;
        };

        let Some(last_segment) = type_path.path.segments.last() else {
            return false;
        };

        // Only check Result types for now
        if last_segment.ident != "Result" {
            return false;
        }

        let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments else {
            return false;
        };

        let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() else {
            return false;
        };

        Self::is_unit_type(inner_type)
    }

    /// Check if a type is the unit type ()
    fn is_unit_type(ty: &syn::Type) -> bool {
        matches!(ty, syn::Type::Tuple(tuple) if tuple.elems.is_empty())
    }

    /// Validate that Unit type is not used with CRUD operations (SELECT/INSERT/UPDATE/DELETE)
    fn validate_unit_type_not_allowed_in_crud(
        return_type: &syn::Type,
        sql_type: &SqlStatementType,
        span: proc_macro2::Span,
        has_unchecked: bool,
    ) -> syn::Result<()> {
        if !Self::return_type_contains_unit(return_type) {
            return Ok(());
        }

        if Self::is_crud_operation(sql_type) && !has_unchecked {
            return Err(syn::Error::new(
                span,
                "Unit type () not allowed with SELECT/INSERT/UPDATE/DELETE operations. These should return data or QueryResult",
            ));
        }

        Ok(())
    }

    /// Validate that Stream return types cannot be declared as async functions
    fn validate_stream_not_async(
        return_type: &syn::Type,
        is_async: bool,
        span: proc_macro2::Span,
    ) -> syn::Result<()> {
        // Check if return type is impl Stream<...>
        let is_stream_type = matches!(return_type, syn::Type::ImplTrait(impl_trait)
            if impl_trait.bounds.iter().any(|bound| {
                if let syn::TypeParamBound::Trait(trait_bound) = bound {
                    crate::type_analyzer::TypeAnalyzer::path_ends_with(&trait_bound.path, "Stream")
                } else {
                    false
                }
            })
        );

        if is_stream_type && is_async {
            return Err(syn::Error::new(
                span,
                "Stream return types cannot be declared as async functions. Remove 'async' keyword - streams are returned directly without await",
            ));
        }

        Ok(())
    }

    /// Run all validations that can be performed before SQL parsing
    fn run_pre_parsing_validations(
        return_type: &syn::Type,
        method: &syn::TraitItemFn,
        _has_unchecked: bool,
    ) -> syn::Result<()> {
        // Validate Unit type requires unchecked flag
        // TEMPORARILY COMMENTED OUT - evaluating behavior without this validation
        // Self::validate_unit_type_requires_unchecked(
        //     return_type,
        //     has_unchecked,
        //     method.sig.span(),
        // )?;

        //Validate Stream methods cannot be async
        Self::validate_stream_not_async(
            return_type,
            method.sig.asyncness.is_some(),
            method.sig.span(),
        )?;

        Ok(())
    }

    /// Run all validations that require SQL parsing results
    fn run_post_parsing_validations(
        return_type: &syn::Type,
        method: &syn::TraitItemFn,
        kind: &SqlStatementType,
        has_unchecked: bool,
    ) -> syn::Result<()> {
        // Validate unknown SQL statements require unchecked
        // Self::validate_unknown_sql_requires_unchecked(
        //     kind,
        //     has_unchecked,
        //     method.sig.span(),
        // )?;

        // Validate DDL commands are not allowed in DML
        Self::validate_no_ddl_in_dml(kind, method.sig.span())?;

        // Validate Unit type not allowed in CRUD operations
        Self::validate_unit_type_not_allowed_in_crud(
            return_type,
            kind,
            method.sig.span(),
            has_unchecked,
        )?;

        Ok(())
    }

    fn detect_sql_type(statement_opt: Option<&Arc<Statement>>) -> SqlStatementType {
        match statement_opt {
            Some(statement) => match statement.as_ref() {
                Statement::Query(_) => SqlStatementType::Select,
                Statement::Insert(_) => SqlStatementType::Insert,
                Statement::Update(_) => SqlStatementType::Update,
                Statement::Delete(_) => SqlStatementType::Delete,
                Statement::CreateTable(_) => SqlStatementType::DDL,
                Statement::CreateIndex { .. } => SqlStatementType::DDL,
                Statement::CreateView { .. } => SqlStatementType::DDL,
                Statement::CreateVirtualTable { .. } => SqlStatementType::DDL,
                Statement::CreateSchema { .. } => SqlStatementType::DDL,
                Statement::CreateDatabase { .. } => SqlStatementType::DDL,
                Statement::Drop { .. } => SqlStatementType::DDL,
                Statement::AlterTable { .. } => SqlStatementType::DDL,
                Statement::AlterView { .. } => SqlStatementType::DDL,
                Statement::AlterIndex { .. } => SqlStatementType::DDL,
                Statement::Truncate { .. } => SqlStatementType::DDL,
                Statement::Pragma { .. } => SqlStatementType::DDL,
                _ => SqlStatementType::Unknown,
            },
            None => SqlStatementType::Unknown,
        }
    }

    /// Validate that DDL commands are not used with DML macro
    fn validate_no_ddl_in_dml(kind: &SqlStatementType, span: proc_macro2::Span) -> syn::Result<()> {
        if matches!(kind, SqlStatementType::DDL) {
            return Err(syn::Error::new(
                span,
                "DDL commands (CREATE, DROP, ALTER, PRAGMA, etc.) are not allowed with #[dml]. Use #[dml(\"SQL\", unchecked)] if this is intentional, or consider using a different approach for DDL operations.",
            ));
        }
        Ok(())
    }

    /// Check if a SQL statement type is a CRUD operation
    fn is_crud_operation(sql_type: &SqlStatementType) -> bool {
        matches!(
            sql_type,
            SqlStatementType::Select
                | SqlStatementType::Insert
                | SqlStatementType::Update
                | SqlStatementType::Delete
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{Generics, parse_quote};

    #[test]
    fn test_detects_impl_executor() {
        let generics: Generics = parse_quote! {};
        let ty: syn::Type = parse_quote! { impl sqlx::Executor<'_, Database = sqlx::Sqlite> };

        assert!(DmlParser::is_sqlx_executor_type(&ty, &generics));
    }

    #[test]
    fn test_detects_generic_executor() {
        let mut generics: Generics = Generics::default();
        let where_clause: syn::WhereClause = parse_quote! {
            where EX: sqlx::Executor<'e, Database = sqlx::Sqlite>
        };
        generics.where_clause = Some(where_clause);

        let ty: syn::Type = parse_quote! { EX };

        assert!(DmlParser::is_sqlx_executor_type(&ty, &generics));
    }

    #[test]
    fn test_detects_pool_reference() {
        let generics: Generics = parse_quote! {};
        let ty: syn::Type = parse_quote! { &Pool<sqlx::Sqlite> };

        assert!(DmlParser::is_sqlx_executor_type(&ty, &generics));
    }

    #[test]
    fn test_rejects_non_executor() {
        let generics: Generics = parse_quote! {};
        let ty: syn::Type = parse_quote! { String };

        assert!(!DmlParser::is_sqlx_executor_type(&ty, &generics));
    }

    #[test]
    fn test_rejects_generic_without_executor_bound() {
        let mut generics: Generics = Generics::default();
        let where_clause: syn::WhereClause = parse_quote! {
            where T: ToString
        };
        generics.where_clause = Some(where_clause);

        let ty: syn::Type = parse_quote! { T };

        assert!(!DmlParser::is_sqlx_executor_type(&ty, &generics));
    }

    #[test]
    fn test_debug_generic_t_tostring() {
        let mut generics: Generics = Generics::default();
        let where_clause: syn::WhereClause = parse_quote! {
            where T: ToString
        };
        generics.where_clause = Some(where_clause);

        let ty: syn::Type = parse_quote! { T };

        println!("Testing T with ToString bound...");
        let result = DmlParser::is_sqlx_executor_type(&ty, &generics);
        println!("Result: {}", result);

        // Este deveria ser FALSE - T: ToString NÃO é um executor
        assert!(!result, "T: ToString should NOT be detected as executor");
    }

    #[test]
    fn test_inline_bound_executor() {
        // Test the specific case: <'e, E: Executor<'e>>
        let generics: syn::Generics = parse_quote! { <'e, E: Executor<'e>> };
        let ty: syn::Type = parse_quote! { E };

        let result = DmlParser::is_sqlx_executor_type(&ty, &generics);
        assert!(
            result,
            "E: Executor<'e> inline bound should be detected as executor"
        );
    }

    #[test]
    fn test_stream_return_types() {
        // Test Stream<Item = Result<(i64, String)>>
        let stream_tuple_sig: syn::TraitItemFn = parse_quote! {
            fn test(&self) -> impl Stream<Item = Result<(i64, String)>>;
        };
        let stream_tuple_method = create_test_method(stream_tuple_sig);

        assert!(
            stream_tuple_method.is_stream_type(),
            "Should detect Stream type"
        );
        assert!(
            stream_tuple_method.is_tuple_type(),
            "Should detect tuple in Stream"
        );
        assert!(
            stream_tuple_method.get_ok_type().is_some(),
            "Should extract tuple from Result"
        );
        assert!(
            stream_tuple_method.parse_tuple_types().is_ok(),
            "Should parse tuple types"
        );

        // Test Stream<Item = Result<User>>
        let stream_struct_sig: syn::TraitItemFn = parse_quote! {
            fn test(&self) -> impl Stream<Item = Result<User>>;
        };
        let stream_struct_method = create_test_method(stream_struct_sig);

        assert!(
            stream_struct_method.is_stream_type(),
            "Should detect Stream type"
        );
        assert!(
            !stream_struct_method.is_tuple_type(),
            "Should not detect tuple for struct"
        );
        assert!(
            stream_struct_method.get_ok_type().is_some(),
            "Should extract struct from Result"
        );

        // Test Stream<Item = Result<i32>> (scalar)
        let stream_scalar_sig: syn::TraitItemFn = parse_quote! {
            fn test(&self) -> impl Stream<Item = Result<i32>>;
        };
        let stream_scalar_method = create_test_method(stream_scalar_sig);

        assert!(
            stream_scalar_method.is_stream_type(),
            "Should detect Stream type"
        );
        assert!(
            !stream_scalar_method.is_tuple_type(),
            "Should not detect tuple for scalar"
        );
        assert!(
            stream_scalar_method.get_ok_type().is_some(),
            "Should extract scalar from Result"
        );
    }

    #[test]
    fn test_pagination_return_types() {
        // Test Result<Serial<User>>
        let serial_sig: syn::TraitItemFn = parse_quote! {
            fn test(&self) -> Result<Serial<User>>;
        };
        let serial_method = create_test_method(serial_sig);

        assert!(!serial_method.is_stream_type(), "Should not detect Stream");
        assert!(
            !serial_method.is_tuple_type(),
            "Should not detect tuple for pagination"
        );
        assert!(
            serial_method.is_pagination_type(),
            "Should detect pagination type"
        );
        assert_eq!(
            serial_method.return_ok_type_name(),
            Some("Serial".to_string()),
            "Should extract Serial name"
        );

        // Test Result<Slice<User>>
        let slice_sig: syn::TraitItemFn = parse_quote! {
            fn test(&self) -> Result<Slice<User>>;
        };
        let slice_method = create_test_method(slice_sig);

        assert!(!slice_method.is_stream_type(), "Should not detect Stream");
        assert!(
            !slice_method.is_tuple_type(),
            "Should not detect tuple for pagination"
        );
        assert!(
            slice_method.is_pagination_type(),
            "Should detect pagination type"
        );
        assert_eq!(
            slice_method.return_ok_type_name(),
            Some("Slice".to_string()),
            "Should extract Slice name"
        );

        // Test Result<Cursor<User>>
        let cursor_sig: syn::TraitItemFn = parse_quote! {
            fn test(&self) -> Result<Cursor<User>>;
        };
        let cursor_method = create_test_method(cursor_sig);

        assert!(!cursor_method.is_stream_type(), "Should not detect Stream");
        assert!(
            !cursor_method.is_tuple_type(),
            "Should not detect tuple for pagination"
        );
        assert!(
            cursor_method.is_pagination_type(),
            "Should detect pagination type"
        );
        assert_eq!(
            cursor_method.return_ok_type_name(),
            Some("Cursor".to_string()),
            "Should extract Cursor name"
        );

        // Test Result<Serial<(i64, String)>> (tuple pagination)
        let serial_tuple_sig: syn::TraitItemFn = parse_quote! {
            fn test(&self) -> Result<Serial<(i64, String)>>;
        };
        let serial_tuple_method = create_test_method(serial_tuple_sig);

        assert!(
            !serial_tuple_method.is_stream_type(),
            "Should not detect Stream"
        );
        assert!(
            serial_tuple_method.is_tuple_type(),
            "Should detect tuple in Serial"
        );
        assert!(
            serial_tuple_method.is_pagination_type(),
            "Should detect pagination type"
        );
        assert_eq!(
            serial_tuple_method.return_ok_type_name(),
            Some("Serial".to_string()),
            "Should extract Serial name"
        );
        assert!(
            serial_tuple_method.parse_tuple_types().is_ok(),
            "Should parse tuple from Serial"
        );
    }

    #[test]
    fn test_struct_and_tuple_return_types() {
        // Test Result<User>
        let struct_sig: syn::TraitItemFn = parse_quote! {
            fn test(&self) -> Result<User>;
        };
        let struct_method = create_test_method(struct_sig);

        assert!(!struct_method.is_stream_type(), "Should not detect Stream");
        assert!(
            !struct_method.is_tuple_type(),
            "Should not detect tuple for struct"
        );
        assert!(
            !struct_method.is_pagination_type(),
            "Should not detect pagination"
        );
        assert_eq!(
            struct_method.return_ok_type_name(),
            Some("User".to_string()),
            "Should extract User name"
        );

        // Test Result<(i64, String)>
        let tuple_sig: syn::TraitItemFn = parse_quote! {
            fn test(&self) -> Result<(i64, String)>;
        };
        let tuple_method = create_test_method(tuple_sig);

        assert!(!tuple_method.is_stream_type(), "Should not detect Stream");
        assert!(
            tuple_method.is_tuple_type(),
            "Should detect tuple in Result"
        );
        assert!(
            !tuple_method.is_pagination_type(),
            "Should not detect pagination"
        );
        assert!(
            tuple_method.parse_tuple_types().is_ok(),
            "Should parse tuple from Result"
        );

        // Test Result<Vec<User>>
        let vec_sig: syn::TraitItemFn = parse_quote! {
            fn test(&self) -> Result<Vec<User>>;
        };
        let vec_method = create_test_method(vec_sig);

        assert!(!vec_method.is_stream_type(), "Should not detect Stream");
        assert!(
            !vec_method.is_tuple_type(),
            "Should not detect tuple for Vec"
        );
        assert!(
            !vec_method.is_pagination_type(),
            "Should not detect pagination for Vec"
        );
        assert_eq!(
            vec_method.return_ok_type_name(),
            Some("Vec".to_string()),
            "Should extract Vec name"
        );

        // Test Result<Option<User>>
        let option_sig: syn::TraitItemFn = parse_quote! {
            fn test(&self) -> Result<Option<User>>;
        };
        let option_method = create_test_method(option_sig);

        assert!(!option_method.is_stream_type(), "Should not detect Stream");
        assert!(
            !option_method.is_tuple_type(),
            "Should not detect tuple for Option"
        );
        assert!(
            !option_method.is_pagination_type(),
            "Should not detect pagination for Option"
        );
        assert_eq!(
            option_method.return_ok_type_name(),
            Some("Option".to_string()),
            "Should extract Option name"
        );
    }

    #[test]
    fn test_scalar_return_types() {
        // Test Result<i32>
        let int_sig: syn::TraitItemFn = parse_quote! {
            fn test(&self) -> Result<i32>;
        };
        let int_method = create_test_method(int_sig);

        assert!(!int_method.is_stream_type(), "Should not detect Stream");
        assert!(
            !int_method.is_tuple_type(),
            "Should not detect tuple for scalar"
        );
        assert!(
            !int_method.is_pagination_type(),
            "Should not detect pagination for scalar"
        );
        assert_eq!(
            int_method.return_ok_type_name(),
            Some("i32".to_string()),
            "Should extract i32 name"
        );

        // Test Result<String>
        let string_sig: syn::TraitItemFn = parse_quote! {
            fn test(&self) -> Result<String>;
        };
        let string_method = create_test_method(string_sig);

        assert!(!string_method.is_stream_type(), "Should not detect Stream");
        assert!(
            !string_method.is_tuple_type(),
            "Should not detect tuple for String"
        );
        assert!(
            !string_method.is_pagination_type(),
            "Should not detect pagination for String"
        );
        assert_eq!(
            string_method.return_ok_type_name(),
            Some("String".to_string()),
            "Should extract String name"
        );

        // Test Result<()> (unit type)
        let unit_sig: syn::TraitItemFn = parse_quote! {
            fn test(&self) -> Result<()>;
        };
        let unit_method = create_test_method(unit_sig);

        assert!(!unit_method.is_stream_type(), "Should not detect Stream");
        assert!(
            unit_method.is_tuple_type(),
            "Should detect tuple for unit (empty tuple)"
        );
        assert!(
            !unit_method.is_pagination_type(),
            "Should not detect pagination for unit"
        );
    }

    #[test]
    fn test_complex_nested_types() {
        // Test Result<Vec<(i64, Option<String>)>>
        let complex_tuple_sig: syn::TraitItemFn = parse_quote! {
            fn test(&self) -> Result<Vec<(i64, Option<String>)>>;
        };
        let complex_tuple_method = create_test_method(complex_tuple_sig);

        assert!(
            !complex_tuple_method.is_stream_type(),
            "Should not detect Stream"
        );
        assert!(
            complex_tuple_method.is_tuple_type(),
            "Should detect tuple in Vec"
        );
        assert!(
            !complex_tuple_method.is_pagination_type(),
            "Should not detect pagination for Vec"
        );
        assert_eq!(
            complex_tuple_method.return_ok_type_name(),
            Some("Vec".to_string()),
            "Should extract Vec name"
        );

        // Test Result<Option<(i64, String)>>
        let option_tuple_sig: syn::TraitItemFn = parse_quote! {
            fn test(&self) -> Result<Option<(i64, String)>>;
        };
        let option_tuple_method = create_test_method(option_tuple_sig);

        assert!(
            !option_tuple_method.is_stream_type(),
            "Should not detect Stream"
        );
        assert!(
            option_tuple_method.is_tuple_type(),
            "Should detect tuple in Option"
        );
        assert!(
            !option_tuple_method.is_pagination_type(),
            "Should not detect pagination for Option"
        );
        assert_eq!(
            option_tuple_method.return_ok_type_name(),
            Some("Option".to_string()),
            "Should extract Option name"
        );
    }

    fn create_test_method(method_sig: syn::TraitItemFn) -> DmlMethod {
        DmlMethod {
            method: method_sig,
            sql_content: "SELECT test".to_string(),
            parameters: vec![],
            statement: None,
            kind: sqlx_data_parser::SqlStatementType::Select,
            is_json_query: false,
            is_multi_insert: false,
            is_unchecked: false,
            has_explicit_instrument: false,
            trait_instrument: false,
            return_info_cache: std::sync::OnceLock::new(),
        }
    }

    #[test]
    fn test_cache_behavior() {
        // Test that cache is working and consistent
        let stream_sig: syn::TraitItemFn = parse_quote! {
            fn test(&self) -> impl Stream<Item = Result<(i64, String)>>;
        };
        let method = create_test_method(stream_sig);

        // First access should initialize cache
        let is_tuple_1 = method.is_tuple_type();
        let is_stream_1 = method.is_stream_type();
        let ok_type_1 = method.get_ok_type().is_some();

        // Second access should use cached values
        let is_tuple_2 = method.is_tuple_type();
        let is_stream_2 = method.is_stream_type();
        let ok_type_2 = method.get_ok_type().is_some();

        // Results should be consistent
        assert_eq!(
            is_tuple_1, is_tuple_2,
            "Cache should be consistent for is_tuple_type"
        );
        assert_eq!(
            is_stream_1, is_stream_2,
            "Cache should be consistent for is_stream_type"
        );
        assert_eq!(
            ok_type_1, ok_type_2,
            "Cache should be consistent for get_ok_type"
        );

        // Values should be correct
        assert!(is_tuple_1, "Should detect tuple");
        assert!(is_stream_1, "Should detect stream");
        assert!(ok_type_1, "Should have ok_type");
    }

    #[cfg(any(feature = "postgres", feature = "sqlite"))]
    #[test]
    fn test_convert_named_to_positional_in_sql() {
        // Create test parameters
        let parameters = vec![
            DmlParameter {
                name: "name".to_string(),
                type_: syn::parse_quote! { String },
                is_pool: false,
                is_dynamic_param: false,
                is_generic: false,
            },
            DmlParameter {
                name: "age".to_string(),
                type_: syn::parse_quote! { i32 },
                is_pool: false,
                is_dynamic_param: false,
                is_generic: false,
            },
            DmlParameter {
                name: "pool".to_string(),
                type_: syn::parse_quote! { &Pool<Sqlite> },
                is_pool: true,
                is_dynamic_param: false,
                is_generic: false,
            },
        ];

        // Test basic named parameter conversion
        let sql = "SELECT * FROM users WHERE name = @name AND age = @age";
        let result = DmlParser::convert_named_to_positional_in_sql(sql, &parameters).unwrap();

        // Should convert to numbered placeholders (for PostgreSQL/SQLite)
        let expected = "SELECT * FROM users WHERE name = $1 AND age = $2";
        assert_eq!(
            result, expected,
            "Should convert named params to numbered placeholders"
        );

        // Test SQL without named parameters
        let sql_no_params = "SELECT * FROM users";
        let result_no_params =
            DmlParser::convert_named_to_positional_in_sql(sql_no_params, &parameters).unwrap();
        assert_eq!(
            result_no_params, sql_no_params,
            "Should not modify SQL without named params"
        );

        // Test SQL with complex formatting
        let sql_complex = "INSERT INTO users (name, age) VALUES (@name, @age)";
        let result_complex =
            DmlParser::convert_named_to_positional_in_sql(sql_complex, &parameters).unwrap();
        let expected_complex = "INSERT INTO users (name, age) VALUES ($1, $2)";
        assert_eq!(
            result_complex, expected_complex,
            "Should handle complex SQL formatting"
        );

        // Test duplicate parameter references
        let sql_duplicate = "UPDATE users SET name = @name WHERE old_name = @name";
        let result_duplicate =
            DmlParser::convert_named_to_positional_in_sql(sql_duplicate, &parameters).unwrap();
        let expected_duplicate = "UPDATE users SET name = $1 WHERE old_name = $1";
        assert_eq!(
            result_duplicate, expected_duplicate,
            "Should handle duplicate parameter references"
        );

        // Test unknown parameter error
        let sql_unknown = "SELECT * FROM users WHERE unknown = @unknown";
        let result_unknown =
            DmlParser::convert_named_to_positional_in_sql(sql_unknown, &parameters);
        assert!(
            result_unknown.is_err(),
            "Should error on unknown parameters"
        );

        let error_msg = result_unknown.unwrap_err().to_string();
        assert!(
            error_msg.contains("Unknown named parameters"),
            "Error should mention unknown parameters"
        );
        assert!(
            error_msg.contains("unknown"),
            "Error should mention the specific unknown parameter"
        );
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn test_convert_named_to_positional_in_sql_for_mysql() {
        // Create test parameters
        let parameters = vec![
            DmlParameter {
                name: "name".to_string(),
                type_: syn::parse_quote! { String },
                is_pool: false,
                is_dynamic_param: false,
                is_generic: false,
            },
            DmlParameter {
                name: "age".to_string(),
                type_: syn::parse_quote! { i32 },
                is_pool: false,
                is_dynamic_param: false,
                is_generic: false,
            },
            DmlParameter {
                name: "pool".to_string(),
                type_: syn::parse_quote! { &Pool<MySql> },
                is_pool: true,
                is_dynamic_param: false,
                is_generic: false,
            },
        ];

        // Test basic named parameter conversion
        let sql = "SELECT * FROM users WHERE name = @name AND age = @age";
        let result = DmlParser::convert_named_to_positional_in_sql(sql, &parameters).unwrap();

        // MySQL uses ? as positional placeholder
        let expected = "SELECT * FROM users WHERE name = ? AND age = ?";
        assert_eq!(
            result, expected,
            "Should convert named params to ? placeholders for MySQL"
        );

        // Test SQL without named parameters
        let sql_no_params = "SELECT * FROM users";
        let result_no_params =
            DmlParser::convert_named_to_positional_in_sql(sql_no_params, &parameters).unwrap();
        assert_eq!(
            result_no_params, sql_no_params,
            "Should not modify SQL without named params"
        );

        // Test SQL with complex formatting
        let sql_complex = "INSERT INTO users (name, age) VALUES (@name, @age)";
        let result_complex =
            DmlParser::convert_named_to_positional_in_sql(sql_complex, &parameters).unwrap();
        let expected_complex = "INSERT INTO users (name, age) VALUES (?, ?)";
        assert_eq!(
            result_complex, expected_complex,
            "Should handle complex SQL formatting with ? placeholders"
        );

        // Test duplicate parameter references
        let sql_duplicate = "UPDATE users SET name = @name WHERE old_name = @name";
        let result_duplicate =
            DmlParser::convert_named_to_positional_in_sql(sql_duplicate, &parameters).unwrap();
        let expected_duplicate = "UPDATE users SET name = ? WHERE old_name = ?";
        assert_eq!(
            result_duplicate, expected_duplicate,
            "Should reuse the same ? for duplicate parameter references"
        );

        // Test unknown parameter error
        let sql_unknown = "SELECT * FROM users WHERE unknown = @unknown";
        let result_unknown =
            DmlParser::convert_named_to_positional_in_sql(sql_unknown, &parameters);
        assert!(
            result_unknown.is_err(),
            "Should error on unknown parameters"
        );

        let error_msg = result_unknown.unwrap_err().to_string();
        assert!(
            error_msg.contains("Unknown named parameters"),
            "Error should mention unknown parameters"
        );
        assert!(
            error_msg.contains("unknown"),
            "Error should mention the specific unknown parameter"
        );
    }
}
