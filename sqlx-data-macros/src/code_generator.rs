//! Unified code generation architecture for sqlx-data macros
//!
//! This module consolidates code generation logic that was previously scattered
//! across the dml.rs module into a structured, maintainable system.

use crate::constants::pagination;
use crate::dml::DmlMethod;
use crate::error::core_error;
use crate::type_analyzer::{
    TypeAnalyzer, TypeCastingAnalyzer, clean_sqlx_cast_syntax_for_runtime, extract_column_name,
    extract_sqlx_explicit_type, has_explicit_sqlx_type,
};
use crate::type_system::{FetchMethod, QueryType};
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote, quote_spanned};

/// Complete context for code generation
#[derive(Clone)]
pub struct GenerationContext<'a> {
    pub method: &'a DmlMethod,
    pub query_type: QueryType,
    pub fetch_method: FetchMethod,
    pub pool_expr: TokenStream,
    pub fetch_call: TokenStream,
    pub param_names: Vec<syn::Ident>,
}

/// Types of parameter wrappers that can be applied
#[derive(Debug, Clone, PartialEq)]
enum ParamWrapper {
    None,       // No wrapper needed
    Json,       // Wrap with sqlx::types::Json()
    BytesToVec, // Convert Bytes to Vec<u8>
}

/// Main code generator that coordinates all code generation tasks
pub struct CodeGenerator;

impl CodeGenerator {
    /// Generate list of parameters to skip in tracing instrumentation
    #[cfg(feature = "tracing")]
    fn generate_tracing_skip_list(method: &DmlMethod) -> Vec<proc_macro2::Ident> {
        let mut skip_params = vec![format_ident!("self")];

        for param in &method.parameters {
            let param_name = &param.name;
            let type_ref = &param.type_;
            let type_str = quote!(#type_ref).to_string();

            // Check parameter type for problematic patterns
            let should_skip = Self::should_skip_param_for_tracing(&type_str);

            if should_skip {
                skip_params.push(format_ident!("{}", param_name));
            }
        }

        skip_params
    }

    /// Check if parameter type should be skipped in tracing
    #[cfg(feature = "tracing")]
    fn should_skip_param_for_tracing(type_str: &str) -> bool {
        // Remove spaces for pattern matching since quote! adds spaces
        let normalized = type_str.replace(" ", "");

        let problematic_patterns = [
            "implInto<",
            "implAsRef<",
            "implAsMut<",
            "implDeref<",
            "implDerefMut<",
            "Connection",
            "Pool",
            "Executor",
            "Transaction",
            "&mut",
            "Vec<",
            "HashMap<",
            "BTreeMap<",
            "Bytes",
            "BytesMut",
            "[u8",
            "Blob",
        ];

        for pattern in &problematic_patterns {
            if normalized.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Generate complete DML implementation methods
    pub fn generate_dml_methods(method: &DmlMethod) -> syn::Result<TokenStream> {
        let query_method = Self::generate_query_method(method)?;
        let default_method = Self::generate_default_method_with_docs(method, &query_method)?;

        Ok(quote_spanned! { method.method_span() =>
            #query_method
            #default_method
        })
    }

    /// Generate the _query method that does the actual SQLx work
    fn generate_query_method(method: &DmlMethod) -> syn::Result<TokenStream> {
        let query_method_name = format_ident!("{}_query", method.name());
        let return_type = &method.return_type();
        let params = MethodSignatureGenerator::generate_query_method_params(method);
        let async_keyword = MethodSignatureGenerator::generate_async_keyword(method);
        let sqlx_call = SqlxCallGenerator::emit_sqlx_call(method)?;

        let (impl_generics, _ty_generics, where_clause) = method.generics().split_for_impl();

        let instrument_attr = Self::generate_instrument_attribute(method);

        Ok(quote_spanned! { method.method_span() =>
            #instrument_attr
            #async_keyword fn #query_method_name #impl_generics (&self, #(#params),*) -> #return_type #where_clause {
                #sqlx_call
            }
        })
    }

    /// Generate tracing instrument attribute for methods
    fn generate_instrument_attribute(method: &DmlMethod) -> TokenStream {
        if !method.trait_instrument && !method.has_explicit_instrument {
            return quote! {};
        }

        #[cfg(feature = "tracing")]
        {
            let skip_params = Self::generate_tracing_skip_list(method);
            if skip_params.is_empty() {
                quote! { #[tracing::instrument] }
            } else {
                quote! { #[tracing::instrument(skip(#(#skip_params),*))] }
            }
        }
        #[cfg(not(feature = "tracing"))]
        {
            quote! {}
        }
    }

    /// Generate the default implementation method with generated code documentation
    fn generate_default_method_with_docs(
        method: &DmlMethod,
        query_method: &TokenStream,
    ) -> syn::Result<TokenStream> {
        let method_name = format_ident!("{}", method.name());
        let query_method_name = format_ident!("{}_query", method.name());
        let return_type = &method.return_type();
        let params = MethodSignatureGenerator::generate_method_params(method);
        let all_param_names = MethodSignatureGenerator::generate_all_param_names(method);
        let async_keyword = MethodSignatureGenerator::generate_async_keyword(method);

        let (impl_generics, _ty_generics, where_clause) = method.generics().split_for_impl();

        // Generate documentation showing the generated query method code
        let query_code = DocumentationGenerator::format_generated_code(query_method);
        let doc_comment = DocumentationGenerator::generate_doc_comment(&query_code);

        // For Stream methods, don't add .await since they return Stream directly
        let call_expr = if method.is_stream_type() {
            quote! { self.#query_method_name(#(#all_param_names),*) }
        } else {
            quote! { self.#query_method_name(#(#all_param_names),*).await }
        };

        Ok(quote_spanned! { method.method_span() =>
            #[doc = #doc_comment]
            #async_keyword fn #method_name #impl_generics (&self, #(#params),*) -> #return_type #where_clause {
                #call_expr
            }
        })
    }
}

/// Generates method-related code (parameters, names, keywords)
pub struct MethodSignatureGenerator;

impl MethodSignatureGenerator {
    /// Helper function to generate method parameters for public methods (all parameters including pool)
    pub fn generate_method_params(method: &DmlMethod) -> Vec<proc_macro2::TokenStream> {
        let params: Vec<_> = method
            .parameters
            .iter()
            .map(|p| {
                let name = format_ident!("{}", p.name);
                let type_ = &p.type_;
                quote! { #name: #type_ }
            })
            .collect();

        params
    }

    /// Helper function to generate method parameters for _query methods with parameter decoration
    pub fn generate_query_method_params(method: &DmlMethod) -> Vec<proc_macro2::TokenStream> {
        let needs_decoration = method.is_data_modification();

        method
            .parameters
            .iter()
            .map(|p| {
                let name = format_ident!("{}", p.name);
                let should_transform = needs_decoration && !p.is_pool && !p.is_dynamic_param;
                let transformed_type =
                    FetchCallGenerator::transform_param_type(&p.type_, should_transform, method);
                quote! { #name: #transformed_type }
            })
            .collect()
    }

    /// Helper function to generate parameter names for calling (only query params)
    pub fn generate_param_names(method: &DmlMethod) -> Vec<syn::Ident> {
        method
            .parameters
            .iter()
            .filter(|p| !p.is_pool && !p.is_dynamic_param) // Exclude pool and dynamic parameters
            .map(|p| format_ident!("{}", p.name))
            .collect()
    }

    /// Generate initial_binds for build_dynamic_sql
    pub fn generate_initial_binds(method: &DmlMethod) -> TokenStream {
        let query_params: Vec<_> = method
            .parameters
            .iter()
            .filter(|p| !p.is_pool && !p.is_dynamic_param)
            .enumerate()
            .collect();

        if query_params.is_empty() {
            quote! { Vec::with_capacity(0) }
        } else {
            let param_bindings = query_params.iter().map(|(_index, param)| {
                let param_name = format_ident!("{}", param.name);
                quote! {
                    sqlx_data::FilterValue::from(#param_name)
                }
            });

            quote! {
                [
                    #(#param_bindings),*
                ].into()
            }
        }
    }

    /// Detect tuple-based multi-insert parameter
    /// Returns the parameter if it's an iterable of tuples (e.g., Vec<(i64, String)>, impl IntoIterator<Item, Slice &[])
    pub fn detect_tuple_multi_insert_param(
        method: &DmlMethod,
    ) -> Option<&crate::dml::DmlParameter> {
        method
            .parameters
            .iter()
            .filter(|p| !p.is_pool && !p.is_dynamic_param)
            .find(|p| TypeAnalyzer::is_tuple_iterable_type(&p.type_, method.generics()))
    }

    /// Helper function to generate all parameter names for calling (query + pool)
    pub fn generate_all_param_names(method: &DmlMethod) -> Vec<TokenStream> {
        let needs_decoration = method.is_data_modification();

        method
            .parameters
            .iter()
            .map(|p| {
                let param_name = format_ident!("{}", p.name);
                let should_transform = needs_decoration && !p.is_pool && !p.is_dynamic_param;
                FetchCallGenerator::transform_param_call(
                    &param_name,
                    &p.type_,
                    should_transform,
                    method,
                )
            })
            .collect()
    }

    /// Helper function to generate async keyword
    pub fn generate_async_keyword(method: &DmlMethod) -> proc_macro2::TokenStream {
        if method.is_async() {
            quote! { async }
        } else {
            quote! {}
        }
    }
}

/// Generates SQLx call code based on query strategy
pub struct SqlxCallGenerator;

impl SqlxCallGenerator {
    /// Prepare complete generation context with all analysis results
    pub fn prepare_generation_context(method: &DmlMethod) -> syn::Result<GenerationContext<'_>> {
        let return_type =
            TypeAnalyzer::analyze_type(method.return_type().unwrap_or(&syn::parse_quote! { () }))?;
        let query_type = TypeAnalyzer::determine_query_strategy(&return_type)?;
        let fetch_method = TypeAnalyzer::determine_fetch_method(&return_type);
        let pool_expr = TypeAnalyzer::determine_pool_expr(method);
        let fetch_call = TypeAnalyzer::determine_fetch_call(&fetch_method, &pool_expr);
        let param_names = MethodSignatureGenerator::generate_param_names(method);

        Ok(GenerationContext {
            method,
            query_type,
            fetch_method,
            pool_expr,
            fetch_call,
            param_names,
        })
    }

    /// Generate the appropriate SQLx call using unified type system
    pub fn emit_sqlx_call(method: &DmlMethod) -> syn::Result<TokenStream> {
        let context = Self::prepare_generation_context(method)?;

        match context.query_type {
            _ if Self::should_use_multi_insert_generator(&context) => {
                MultiRowInsertGenerator::generate(&context)
            }
            _ if Self::should_use_stream_generator(&context) => StreamGenerator::generate(&context),
            _ if Self::should_use_paginator_generator(&context) => {
                PaginatedGenerator::generate(&context)
            }
            QueryType::QueryAs => QueryAsGenerator::generate(&context),
            QueryType::QueryScalar => QueryScalarGenerator::generate(&context),
            QueryType::Query => QueryGenerator::generate(&context),
        }
    }

    /// Determine if MultiRowInsertGenerator should be used
    fn should_use_multi_insert_generator(context: &GenerationContext) -> bool {
        context.method.is_multi_insert()
    }

    /// Determine if StreamGenerator should be used
    fn should_use_stream_generator(context: &GenerationContext) -> bool {
        context.method.is_stream_type()
    }

    /// Determine if PaginatedGenerator should be used
    fn should_use_paginator_generator(context: &GenerationContext) -> bool {
        context.method.is_pagination_type()
    }
}

/// Generates multi-row insert calls using build_dynamic_sql
pub struct MultiRowInsertGenerator;

impl MultiRowInsertGenerator {
    pub fn generate(context: &GenerationContext) -> syn::Result<TokenStream> {
        let method = context.method;

        let tuple_param = MethodSignatureGenerator::detect_tuple_multi_insert_param(method)
            .ok_or_else(|| method_error(method, "Multi-insert requires a tuple parameter"))?;

        let validation_code = Self::generate_validation(context, tuple_param)?;
        let query_builder_code = Self::generate_query_builder(context, tuple_param)?;

        Ok(quote_spanned! { method.method_span() =>

            #validation_code
            #query_builder_code

        })
    }

    fn generate_validation(
        context: &GenerationContext,
        tuple_param: &crate::dml::DmlParameter,
    ) -> syn::Result<TokenStream> {
        let method = context.method;
        let sql = &method.sql_content;

        let tuple_types = Self::get_tuple_types(&tuple_param.type_, method)?;

        let var_decls: Vec<_> = tuple_types
            .iter()
            .enumerate()
            .map(|(i, ty)| {
                let var = format_ident!("arg{}", i);
                quote! { let #var: #ty = Default::default(); }
            })
            .collect();

        let binds: Vec<_> = (0..tuple_types.len())
            .map(|i| {
                let var = format_ident!("arg{}", i);
                quote! { #var }
            })
            .collect();

        let inner_type = context.method.get_return_inner_type();

        let (tuple_struct, validation) = match context.query_type {
            QueryType::QueryScalar => {
                (quote! {}, quote! { sqlx::query_scalar!(#sql, #(#binds),*) })
            }
            QueryType::QueryAs if context.method.is_tuple_type() => {
                let tuple_struct = Self::generate_tuple_struct(context)?;
                (
                    tuple_struct,
                    quote! { sqlx::query_as!(QueryTuple, #sql, #(#binds),*) },
                )
            }
            QueryType::QueryAs => (
                quote! {},
                quote! { sqlx::query_as!(#inner_type, #sql, #(#binds),*) },
            ),
            _ => (quote! {}, quote! { sqlx::query!(#sql, #(#binds),*) }),
        };

        Ok(quote! {
            sqlx_data::compile_time_only!({
                #tuple_struct
                #(#var_decls)*
                let _ = #validation;
            });
        })
    }

    fn generate_query_builder(
        context: &GenerationContext,
        tuple_param: &crate::dml::DmlParameter,
    ) -> syn::Result<TokenStream> {
        let param_name = format_ident!("{}", tuple_param.name);

        let method = context.method;
        let stmt = method
            .statement
            .as_ref()
            .ok_or_else(|| error("SQL statement not available"))?;

        if sqlx_data_parser::has_complex_sql_functions_in_values(stmt) {
            return Err(error("SQL functions in VALUES clause not supported"));
        }

        let insert_part =
            sqlx_data_parser::extract_insert_base_from_statement(stmt).map_err(core_error)?;

        let tuple_types = Self::get_tuple_types(&tuple_param.type_, method)?;
        let is_ref = Self::has_reference_in_tuple_items(&tuple_param.type_, method)?;

        let bind_calls: Vec<_> = (0..tuple_types.len())
            .map(|i| {
                let idx = syn::Index::from(i);
                if is_ref {
                    quote! { push_bind(&tuple.#idx) }
                } else {
                    quote! { push_bind(tuple.#idx) }
                }
            })
            .collect();

        let bind_chain = quote! { b.#(#bind_calls).* };

        let on_conflict = sqlx_data_parser::extract_on_conflict_clause_from_statement(stmt)
            .map(|clause| quote! { qb.push(#clause); })
            .unwrap_or_else(|| quote! {});

        let returning = sqlx_data_parser::extract_returning_clause_from_statement(stmt)
            .map(|clause| {
                let cleaned = clean_sqlx_cast_syntax_for_runtime(&clause);
                quote! { qb.push(#cleaned); }
            })
            .unwrap_or_else(|| quote! {});

        let execution = Self::build_execution(context)?;

        // Generate appropriate empty check based on fetch method and query type
        let empty_return = if tuple_param.is_generic {
            // For generic types (impl IntoIterator, generic params), don't generate empty check
            quote! {}
        } else {
            match context.fetch_method {
                FetchMethod::Execute => quote! {
                    if #param_name.is_empty() {
                        return Ok(sqlx_data::QueryResult::default());
                    }
                },
                FetchMethod::FetchAll => match context.query_type {
                    QueryType::QueryScalar | QueryType::QueryAs => quote! {
                        if #param_name.is_empty() {
                            return Ok(Vec::new());
                        }
                    },
                    _ => quote! {
                        if #param_name.is_empty() {
                            return Ok(sqlx_data::QueryResult::default());
                        }
                    },
                },
                FetchMethod::FetchOne => quote! {
                    // No empty check for FetchOne - let it fail naturally
                },
                FetchMethod::FetchOptional => quote! {
                    if #param_name.is_empty() {
                        return Ok(None);
                    }
                },
                _ => quote! {
                    if #param_name.is_empty() {
                        return Ok(sqlx_data::QueryResult::default());
                    }
                },
            }
        };

        Ok(quote! {
            {
                #empty_return
                let mut qb = sqlx::QueryBuilder::<sqlx_data::DB>::new(#insert_part);
                qb.push_values(#param_name, |mut b, tuple| { #bind_chain; });
                #on_conflict
                #returning
                #execution
            }
        })
    }

    fn get_tuple_types(param_type: &syn::Type, method: &DmlMethod) -> syn::Result<Vec<syn::Type>> {
        let inner_type = TypeAnalyzer::get_iterable_inner_type(param_type)
            .cloned()
            .or_else(|| TypeAnalyzer::extract_impl_into_type(param_type))
            .or_else(|| {
                if let syn::Type::Path(path) = param_type {
                    if path.path.segments.len() == 1 {
                        let param_name = path.path.segments[0].ident.to_string();
                        TypeAnalyzer::extract_tuple_from_where_clause(
                            method.generics(),
                            &param_name,
                        )
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .ok_or_else(|| syn::Error::new_spanned(param_type, "Expected tuple parameter"))?;

        match &inner_type {
            syn::Type::Tuple(tuple) => Ok(tuple.elems.iter().cloned().collect()),
            syn::Type::Reference(ref_type) => {
                if let syn::Type::Tuple(tuple) = ref_type.elem.as_ref() {
                    Ok(tuple.elems.iter().cloned().collect())
                } else {
                    Err(syn::Error::new_spanned(
                        &inner_type,
                        "Expected tuple or reference to tuple",
                    ))
                }
            }
            _ => Err(syn::Error::new_spanned(&inner_type, "Expected tuple type")),
        }
    }

    fn build_execution(context: &GenerationContext) -> syn::Result<TokenStream> {
        let fetch_call = &context.fetch_call;
        Ok(match context.fetch_method {
            FetchMethod::Execute => quote! { qb.build().execute(self.get_pool()).await },
            _ => match context.query_type {
                QueryType::QueryScalar => quote! { qb.build_query_scalar()#fetch_call.await },
                QueryType::QueryAs => quote! { qb.build_query_as()#fetch_call.await },
                _ => quote! { qb.build()#fetch_call.await },
            },
        })
    }

    fn generate_tuple_struct(context: &GenerationContext) -> syn::Result<TokenStream> {
        let method = context.method;
        let tuple_types = method.parse_tuple_types()?;
        let inferred = sqlx_data_parser::infer_columns_from_stmt(
            method
                .statement
                .as_ref()
                .ok_or_else(|| error("SQL statement not available"))?,
        )
        .map_err(core_error)?;

        let fields: Vec<_> = inferred
            .columns
            .iter()
            .zip(&tuple_types)
            .map(|(col, ty)| {
                let name = format_ident!("{}", extract_column_name(col));
                quote! { pub #name: #ty }
            })
            .collect();

        Ok(quote! { struct QueryTuple { #(#fields),* } })
    }

    fn has_reference_in_tuple_items(
        param_type: &syn::Type,
        method: &DmlMethod,
    ) -> syn::Result<bool> {
        // Direct reference check
        if matches!(param_type, syn::Type::Reference(_)) {
            return Ok(true);
        }

        // Get the inner tuple type
        let inner_type = TypeAnalyzer::get_iterable_inner_type(param_type)
            .cloned()
            .or_else(|| TypeAnalyzer::extract_impl_into_type(param_type))
            .or_else(|| {
                if let syn::Type::Path(path) = param_type {
                    if path.path.segments.len() == 1 {
                        let param_name = path.path.segments[0].ident.to_string();
                        TypeAnalyzer::extract_tuple_from_where_clause(
                            method.generics(),
                            &param_name,
                        )
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .ok_or_else(|| syn::Error::new_spanned(param_type, "Expected tuple parameter"))?;

        // Check if the inner type is a reference to tuple: &(T1, T2, ...)
        if let syn::Type::Reference(_) = &inner_type {
            return Ok(true);
        }

        Ok(false)
    }
}

/// Generates query_as! calls for struct and tuple types
pub struct QueryAsGenerator;

impl QueryAsGenerator {
    pub fn generate(context: &GenerationContext) -> syn::Result<TokenStream> {
        // Check if it's a tuple type (pagination types are handled earlier in emit_sqlx_call)
        if context.method.is_tuple_type() {
            return Self::generate_tuple_query(context);
        }

        Self::generate_struct_query(context)
    }

    fn generate_tuple_query(context: &GenerationContext) -> syn::Result<TokenStream> {
        let param_names = &context.param_names;
        let method = context.method;
        let fetch_method = &context.fetch_method;
        let fetch_call = &context.fetch_call;
        let tuple_types = method.parse_tuple_types()?;

        // Parse SQL to infer columns
        let inferred = sqlx_data_parser::infer_columns_from_stmt(
            method
                .statement
                .as_ref()
                .ok_or_else(|| error("SQL statement not available"))?,
        )
        .map_err(core_error)?;
        if inferred.columns.len() != tuple_types.len() {
            return Err(error(format!(
                "Tuple has {} elements but SQL query returns {} columns. \
Columns found: {:?}",
                tuple_types.len(),
                inferred.columns.len(),
                inferred.columns
            )));
        }

        let struct_fields =
            TupleConversionGenerator::generate_struct_fields(&inferred.columns, &tuple_types);
        let tuple_conversion_row =
            TupleConversionGenerator::generate_conversion_row(&inferred.columns, &tuple_types);
        let conversion_code =
            TupleConversionGenerator::generate_conversion_code(&tuple_conversion_row, fetch_method);
        let result_var = TupleConversionGenerator::generate_for_fetch(fetch_method);
        let sql = method.sql_content.clone();
        let cleaned_sql = clean_sqlx_cast_syntax_for_runtime(&sql);

        if method.is_unchecked {
            let bind_calls = param_names.iter().map(|param| quote! { .bind(#param) });
            return Ok(quote_spanned! { method.method_span() =>
                #[derive(sqlx::FromRow)]
                struct QueryTuple {
                    #(#struct_fields),*
                }

                let #result_var = sqlx::query_as::<_, QueryTuple>(#cleaned_sql)
                    #(#bind_calls)*
                    #fetch_call
                    .await?;

                #conversion_code
            });
        }

        Ok(quote_spanned! { method.method_span() =>
            struct QueryTuple {
                #(#struct_fields),*
            }

            let #result_var = sqlx::query_as!(QueryTuple, #sql, #(#param_names),*)
                #fetch_call
                .await?;

            #conversion_code
        })
    }

    fn generate_struct_query(context: &GenerationContext) -> syn::Result<TokenStream> {
        let inner_type = context.method.get_return_inner_type();
        let param_names = &context.param_names;
        let method = context.method;
        let fetch_call = &context.fetch_call;
        let sql = method.sql_content.clone();
        let cleaned_sql = clean_sqlx_cast_syntax_for_runtime(&sql);

        if method.is_unchecked {
            let bind_calls = param_names.iter().map(|param| quote! { .bind(#param) });

            // For QueryResult types, use plain sqlx::query() not query_as()
            let target_type_str = inner_type.to_token_stream().to_string();
            if target_type_str.ends_with("QueryResult") {
                return Ok(quote_spanned! { method.method_span() =>
                    sqlx::query(#cleaned_sql)
                        #(#bind_calls)*
                        #fetch_call
                        .await
                });
            }

            // For other struct types, use query_as
            return Ok(quote_spanned! { method.method_span() =>
                sqlx::query_as::<_, #inner_type>(#cleaned_sql)
                    #(#bind_calls)*
                    #fetch_call
                    .await
            });
        }

        Ok(quote_spanned! { method.method_span() =>
            sqlx::query_as!(#inner_type, #sql, #(#param_names),*)
                #fetch_call
                .await
        })
    }
}

/// Generates query_scalar! calls for scalar types
pub struct QueryScalarGenerator;

impl QueryScalarGenerator {
    pub fn generate(context: &GenerationContext) -> syn::Result<TokenStream> {
        // Check if this is a pagination type (Serial<T>, Slice<T>, Cursor<T>)
        if context.method.is_pagination_type() {
            return PaginatedGenerator::generate(context);
        }
        let scalar_type = context.method.get_return_inner_type();
        let fetch_method = &context.fetch_method;
        let param_names = &context.param_names;
        let method = context.method;
        let fetch_call = &context.fetch_call;

        let sql = method.sql_content.clone();
        let cleaned_sql = clean_sqlx_cast_syntax_for_runtime(&sql);

        if method.is_unchecked {
            let bind_calls = param_names.iter().map(|param| quote! { .bind(#param) });
            return Ok(quote_spanned! { method.method_span() =>
                sqlx::query_scalar(#cleaned_sql)
                    #(#bind_calls)*
                    #fetch_call
                    .await
            });
        }

        // Analyze all type requirements once to avoid repeated analysis
        // For scalar queries, check if the first column has explicit SQLx casting
        let has_explicit_casting = method
            .statement
            .as_ref()
            .and_then(|stmt| sqlx_data_parser::infer_columns_from_stmt(stmt).ok())
            .map(|inferred| {
                inferred
                    .columns
                    .first()
                    .map(|column_name| has_explicit_sqlx_type(column_name))
                    .unwrap_or(false)
            })
            .unwrap_or(false);

        let needs_casting =
            !has_explicit_casting && TypeCastingAnalyzer::needs_casting(scalar_type);
        let is_option = TypeCastingAnalyzer::extract_option_type(scalar_type).is_some();
        let should_auto_flatten = Self::should_use_auto_flatten(
            method.return_type().unwrap_or(&syn::parse_quote! { () }),
        );
        let target_type_token =
            if let Some(inner) = TypeCastingAnalyzer::extract_option_type(scalar_type) {
                inner
            } else {
                scalar_type.clone()
            };

        // Separate logic: Vec operations vs single value operations
        match fetch_method {
            FetchMethod::FetchAll => {
                // Vec<T> operations - handle element-wise transformations
                if needs_casting {
                    let is_vec_option = TypeAnalyzer::is_vec_option_type(
                        method.return_type().unwrap_or(&syn::parse_quote! { () }),
                    );
                    let map_expr = if is_vec_option {
                        quote! { |v| v.map(|inner| inner as #target_type_token) }
                    } else {
                        quote! { |v| v as #target_type_token }
                    };

                    Ok(quote_spanned! { method.method_span() =>

                        let value = sqlx::query_scalar!(#sql, #(#param_names),*)
                            #fetch_call
                            .await;
                        Ok(value?.into_iter().map(#map_expr).collect())

                    })
                } else {
                    // Vec without casting - direct SQLx call
                    Ok(quote_spanned! { method.method_span() =>
                        sqlx::query_scalar!(#sql, #(#param_names),*)
                            #fetch_call
                            .await
                    })
                }
            }
            _ => {
                // Single value operations - apply transformations based on flags
                match (needs_casting, should_auto_flatten) {
                    (true, true) => {
                        // Both casting and auto-flatten needed
                        // Use option_cast_expr directly for flatten + casting
                        let cast_expr = option_cast_expr(
                            quote! { value? },
                            scalar_type,
                            false, // is_tuple: this is a scalar value
                        );
                        Ok(quote_spanned! { method.method_span() =>

                            let value = sqlx::query_scalar!(#sql, #(#param_names),*)
                                #fetch_call
                                .await;
                            // Apply flatten + casting for Option<Option<T>> -> Option<T>
                            Ok(#cast_expr)

                        })
                    }
                    (false, true) => {
                        // Auto-flatten only - no casting needed
                        Ok(quote_spanned! { method.method_span() =>

                            let value = sqlx::query_scalar!(#sql, #(#param_names),*)
                                #fetch_call
                                .await;
                            // Apply flatten for Option<Option<T>> -> Option<T>
                            Ok(value?.flatten())

                        })
                    }
                    (true, false) => {
                        // Casting only - no auto-flatten needed
                        let cast_expr = generate_conversion_expr(
                            quote! { value },
                            &target_type_token,
                            is_option,
                            false, // is_tuple: this is a scalar value
                        );
                        Ok(quote_spanned! { method.method_span() =>

                            let value = sqlx::query_scalar!(#sql, #(#param_names),*)
                                #fetch_call
                                .await?;
                            Ok(#cast_expr)

                        })
                    }
                    (false, false) => {
                        // No transformations needed - direct SQLx call
                        Ok(quote_spanned! { method.method_span() =>
                            sqlx::query_scalar!(#sql, #(#param_names),*)
                                #fetch_call
                                .await
                        })
                    }
                }
            }
        }
    }

    /// Check if we should use auto_flatten based on return type signature
    /// Only applies when return type is Result<Option<T>, E>
    fn should_use_auto_flatten(return_type: &syn::Type) -> bool {
        let analyzed = match TypeAnalyzer::analyze_type(return_type) {
            Ok(t) => t,
            Err(_) => {
                log::warn!(
                    "Failed to analyze return type for auto-flatten detection: {:?}",
                    return_type
                );
                return false;
            }
        };

        match analyzed {
            crate::type_system::ReturnType::Result { ok_type, .. } => {
                // Check if ok_type is Option<T>
                matches!(
                    ok_type.as_ref(),
                    crate::type_system::ReturnType::Option { .. }
                )
            }
            _ => false,
        }
    }
}

/// Generates raw query! calls
pub struct QueryGenerator;

impl QueryGenerator {
    pub fn generate(context: &GenerationContext) -> syn::Result<TokenStream> {
        let fetch_method = &context.fetch_method;
        let param_names = &context.param_names;
        let method = context.method;
        let fetch_call = &context.fetch_call;
        let sql = method.sql_content.clone();
        let cleaned_sql = clean_sqlx_cast_syntax_for_runtime(&sql);
        let execute_only = method.is_crud_operation() && !method.is_unchecked;

        match fetch_method {
            FetchMethod::Execute if execute_only => {
                // For execute-only queries returning (), discard the QueryResult
                Ok(quote_spanned! { method.method_span() =>
                    sqlx::query!(#sql, #(#param_names),*)
                        #fetch_call
                        .await
                        .map(|_| ())
                })
            }

            FetchMethod::Execute if method.is_unchecked => {
                // For unchecked execute queries, always map to () since FetchMethod::Execute expects ()
                let bind_calls = param_names.iter().map(|param| quote! { .bind(#param) });
                Ok(quote_spanned! { method.method_span() =>
                    sqlx::query(#cleaned_sql)
                        #(#bind_calls)*
                        #fetch_call
                        .await
                        .map(|_| ())
                })
            }

            _ if method.is_unchecked => {
                // For unchecked non-execute queries, use .map(|_| ()) for compatibility
                let bind_calls = param_names.iter().map(|param| quote! { .bind(#param) });
                Ok(quote_spanned! { method.method_span() =>
                    sqlx::query(#cleaned_sql)
                        #(#bind_calls)*
                        #fetch_call
                        .await
                        .map(|_| ())
                })
            }

            _ => Ok(quote_spanned! { method.method_span() =>
                sqlx::query!(#sql, #(#param_names),*)
                    #fetch_call
                    .await
                    .map(|_| ())
            }),
        }
    }
}

/// Generates .fetch_*() call tokens
pub struct FetchCallGenerator;

impl FetchCallGenerator {
    /// Transform parameter types using declarative pipeline
    fn transform_param_type(
        param_type: &syn::Type,
        needs_decoration: bool,
        method: &DmlMethod,
    ) -> syn::Type {
        // Pipeline: Option<T> → impl Into<T> → Vec<T> → wrapper
        let mut ty = param_type.clone();

        // Strip Option<T> recursively
        if let Some(inner) = TypeCastingAnalyzer::extract_option_type(&ty) {
            let transformed = Self::transform_param_type(&inner, needs_decoration, method);
            return syn::parse_quote! { Option<#transformed> };
        }

        // Strip impl Into<T>
        if let Some(inner) = TypeAnalyzer::extract_impl_into_type(&ty) {
            ty = inner;
        }

        // Transform Vec<T> to &[T] for PostgreSQL arrays (except Vec<u8>, json queries, and Vec<Tuple>)
        if !method.is_json_query
            && let Some(inner) = TypeAnalyzer::get_vec_inner_type(&ty)
                && !TypeAnalyzer::is_vec_u8_type(&ty)
                    && !TypeAnalyzer::is_tuple_iterable_type(&ty, method.generics()) {
                    return syn::parse_quote! { &[#inner] };
                }

        // Apply wrapper if needed
        match needs_decoration {
            true => Self::apply_wrapper_to_type(&ty, Self::get_base_wrapper(&ty, method)),
            false => ty,
        }
    }

    /// Determine wrapper for base types (no Option/impl Into nesting)
    fn get_base_wrapper(param_type: &syn::Type, method: &DmlMethod) -> ParamWrapper {
        // Already decorated? Skip
        if Self::is_already_decorated(param_type) {
            return ParamWrapper::None;
        }

        // Bytes -> Vec<u8>
        if TypeAnalyzer::is_bytes_type(param_type) {
            return ParamWrapper::BytesToVec;
        }

        // Check json flag FIRST - only apply Json decoration if explicitly requested
        if method.is_json_query {
            return ParamWrapper::Json;
        }

        // Scalars -> no decoration
        if let syn::Type::Path(type_path) = param_type
            && TypeAnalyzer::is_scalar(&type_path.path).unwrap_or(false)
        {
            return ParamWrapper::None;
        }

        // String references -> no decoration
        if let syn::Type::Reference(type_ref) = param_type {
            let syn::Type::Path(type_path) = &*type_ref.elem else {
                return ParamWrapper::None; // No Json unless json flag is set
            };
            if TypeAnalyzer::path_ends_with(&type_path.path, "str")
                || TypeAnalyzer::path_ends_with(&type_path.path, "String")
            {
                return ParamWrapper::None;
            }
        }

        ParamWrapper::None
    }

    /// Apply wrapper to a base type
    fn apply_wrapper_to_type(param_type: &syn::Type, wrapper: ParamWrapper) -> syn::Type {
        match wrapper {
            ParamWrapper::Json => syn::parse_quote! { sqlx::types::Json<#param_type> },
            ParamWrapper::BytesToVec => syn::parse_quote! { Vec<u8> },
            ParamWrapper::None => param_type.clone(),
        }
    }

    /// Recursively generate parameter calls for method invocations
    /// Handles: Option<T>, impl Into<T>, Vec<T>, and applies conversions
    fn transform_param_call(
        param_name: &syn::Ident,
        param_type: &syn::Type,
        needs_decoration: bool,
        method: &DmlMethod,
    ) -> TokenStream {
        // Handle Option<T> - only transform if inner type actually needs transformation
        if let Some(inner_type) = TypeCastingAnalyzer::extract_option_type(param_type) {
            // Check if the inner type needs any transformation
            let inner_needs_transformation =
                Self::needs_transformation(&inner_type, needs_decoration, method);

            if inner_needs_transformation {
                // For Option<impl Into<T>>, handle specially
                if let Some(into_inner) = TypeAnalyzer::extract_impl_into_type(&inner_type) {
                    let inner_call = Self::apply_wrapper_to_call(
                        &quote! { inner.into() },
                        &into_inner,
                        needs_decoration,
                        method,
                    );
                    return quote! { #param_name.map(|inner| #inner_call) };
                } else if !method.is_json_query
                    && TypeAnalyzer::get_vec_inner_type(&inner_type).is_some()
                    && !TypeAnalyzer::is_vec_u8_type(&inner_type)
                {
                    // Option<Vec<T>> -> option.as_deref() to get Option<&[T]>
                    return quote! { #param_name.as_deref() };
                } else {
                    // Regular Option<T> that needs decoration
                    let inner_call = Self::apply_wrapper_to_call(
                        &quote! { inner },
                        &inner_type,
                        needs_decoration,
                        method,
                    );
                    return quote! { #param_name.map(|inner| #inner_call) };
                }
            } else {
                // Option<scalar> - no transformation needed
                return quote! { #param_name };
            }
        }

        // Handle impl Into<T> - call .into() and transform the result
        if let Some(inner_type) = TypeAnalyzer::extract_impl_into_type(param_type) {
            let base_call = quote! { #param_name.into() };
            return Self::apply_wrapper_to_call(&base_call, &inner_type, needs_decoration, method);
        }

        // Handle Vec<T> - pass as reference for PostgreSQL arrays (except Vec<u8>, json queries, and Vec<Tuple>)
        if !method.is_json_query
            && TypeAnalyzer::get_vec_inner_type(param_type).is_some()
            && !TypeAnalyzer::is_vec_u8_type(param_type)
            && !TypeAnalyzer::is_tuple_iterable_type(param_type, method.generics())
        {
            return quote! { &#param_name };
        }

        // Base case - apply decoration to direct value
        Self::apply_wrapper_to_call(
            &quote! { #param_name },
            param_type,
            needs_decoration,
            method,
        )
    }

    /// Check if a type needs any transformation (decoration or Into conversion)
    fn needs_transformation(
        param_type: &syn::Type,
        needs_decoration: bool,
        method: &DmlMethod,
    ) -> bool {
        // impl Into<T> always needs transformation (call .into())
        if TypeAnalyzer::extract_impl_into_type(param_type).is_some() {
            return true;
        }

        // Vec<T> needs transformation to &[T] (except Vec<u8>, json queries, and Vec<Tuple>)
        if !method.is_json_query
            && TypeAnalyzer::get_vec_inner_type(param_type).is_some()
            && !TypeAnalyzer::is_vec_u8_type(param_type)
            && !TypeAnalyzer::is_tuple_iterable_type(param_type, method.generics())
        {
            return true;
        }

        // Check if wrapper is needed
        if needs_decoration {
            let wrapper = Self::get_base_wrapper(param_type, method);
            return wrapper != ParamWrapper::None;
        }

        false
    }

    /// Apply wrapper to a call expression
    fn apply_wrapper_to_call(
        call_expr: &TokenStream,
        param_type: &syn::Type,
        needs_decoration: bool,
        method: &DmlMethod,
    ) -> TokenStream {
        if !needs_decoration {
            return call_expr.clone();
        }

        let wrapper = Self::get_base_wrapper(param_type, method);
        match wrapper {
            ParamWrapper::Json => quote! { sqlx::types::Json(#call_expr) },
            ParamWrapper::BytesToVec => quote! { #call_expr.to_vec() },
            ParamWrapper::None => call_expr.clone(),
        }
    }

    /// Check if type is already decorated (Json<T>, Vec<u8>, etc.)
    fn is_already_decorated(param_type: &syn::Type) -> bool {
        TypeAnalyzer::is_already_json_wrapped(param_type)
            || TypeAnalyzer::is_vec_u8_type(param_type)
    }
}

/// Generates streaming queries for impl Stream<Item = Result<T, E>> return types
pub struct StreamGenerator;

impl StreamGenerator {
    pub fn generate(context: &GenerationContext) -> syn::Result<TokenStream> {
        let method = context.method;

        // Extract the inner type from Stream<Item = Result<T, E>> → Result<T, E>
        let stream_item_type = method.get_return_inner_type();

        // Extract T from Result<T, E> → T (the actual data type: User, i32, (i64, String), etc.)
        let actual_data_type = method.get_ok_type().unwrap_or(stream_item_type);

        // Debug: Type extraction working correctly
        // stream_item_type should be Result<T>, actual_data_type should be T

        // Determine if it's tuple, struct, or scalar based on the actual data type
        if method.is_tuple_type() {
            Self::generate_tuple_stream(context, actual_data_type)
        } else if matches!(context.query_type, QueryType::QueryScalar) {
            Self::generate_scalar_stream(context, actual_data_type)
        } else {
            Self::generate_struct_stream(context, actual_data_type)
        }
    }

    fn generate_tuple_stream(
        context: &GenerationContext,
        tuple_type: &syn::Type,
    ) -> syn::Result<TokenStream> {
        let method = context.method;
        let param_names = &context.param_names;
        let fetch_call = &context.fetch_call;
        let sql = &method.sql_content;
        let cleaned_sql = clean_sqlx_cast_syntax_for_runtime(sql);

        // Parse SQL to infer columns for validation
        let inferred = sqlx_data_parser::infer_columns_from_stmt(
            method
                .statement
                .as_ref()
                .ok_or_else(|| error("SQL statement not available"))?,
        )
        .map_err(core_error)?;

        let tuple_types = method.parse_tuple_types()?;
        let struct_fields =
            TupleConversionGenerator::generate_struct_fields(&inferred.columns, &tuple_types);
        //let tuple_conversion_row = TupleConversionGenerator::generate_conversion_row(&inferred.columns, &tuple_types);

        if method.is_unchecked {
            let bind_calls = param_names.iter().map(|param| quote! { .bind(#param) });
            return Ok(quote_spanned! { method.method_span() =>

                sqlx::query_as::<_, #tuple_type>(#cleaned_sql)
                    #(#bind_calls)*
                    #fetch_call
            });
        }

        let bind_calls = param_names.iter().map(|param| quote! { .bind(#param) });

        // Clean SQL for runtime (remove SQLx cast syntax)
        let cleaned_sql = clean_sqlx_cast_syntax_for_runtime(sql);

        Ok(quote_spanned! { method.method_span() =>
            // Compile-time validation - uses original SQL with cast syntax
            sqlx_data::compile_time_only! {
                struct QueryTuple {
                    #(#struct_fields),*
                }
                let _ = sqlx::query_as!(QueryTuple, #sql, #(#param_names),*);
            }

            // Runtime execution - uses cleaned SQL without cast syntax
            sqlx::query_as::<_, #tuple_type>(#cleaned_sql)
                #(#bind_calls)*
                #fetch_call
        })
    }

    fn generate_scalar_stream(
        context: &GenerationContext,
        _scalar_type: &syn::Type,
    ) -> syn::Result<TokenStream> {
        let method = context.method;
        let param_names = &context.param_names;
        let fetch_call = &context.fetch_call;
        let sql = &method.sql_content;
        let cleaned_sql = clean_sqlx_cast_syntax_for_runtime(sql);

        if method.is_unchecked {
            let bind_calls = param_names.iter().map(|param| quote! { .bind(#param) });
            return Ok(quote_spanned! { method.method_span() =>
                sqlx::query_scalar(#cleaned_sql)
                    #(#bind_calls)*
                    #fetch_call
            });
        }

        let bind_calls = param_names.iter().map(|param| quote! { .bind(#param) });

        // Clean SQL for runtime (remove SQLx cast syntax)
        let cleaned_sql = clean_sqlx_cast_syntax_for_runtime(sql);

        Ok(quote_spanned! { method.method_span() =>
            // Compile-time validation - uses original SQL with cast syntax
            sqlx_data::compile_time_only! {
                let _ = sqlx::query_scalar!(#sql, #(#param_names),*);
            }

            // Runtime execution - uses cleaned SQL without cast syntax
            sqlx::query_scalar(#cleaned_sql)
                #(#bind_calls)*
                #fetch_call
        })
    }

    fn generate_struct_stream(
        context: &GenerationContext,
        struct_type: &syn::Type,
    ) -> syn::Result<TokenStream> {
        let method = context.method;
        let param_names = &context.param_names;
        let fetch_call = &context.fetch_call;
        let sql = &method.sql_content;
        let cleaned_sql = clean_sqlx_cast_syntax_for_runtime(sql);

        if method.is_unchecked {
            let bind_calls = param_names.iter().map(|param| quote! { .bind(#param) });
            return Ok(quote_spanned! { method.method_span() =>
                sqlx::query_as::<_, #struct_type>(#cleaned_sql)
                    #(#bind_calls)*
                    #fetch_call
            });
        }

        let bind_calls = param_names.iter().map(|param| quote! { .bind(#param) });

        // Clean SQL for runtime (remove SQLx cast syntax)
        let cleaned_sql = clean_sqlx_cast_syntax_for_runtime(sql);

        Ok(quote_spanned! { method.method_span() =>
            // Compile-time validation - uses original SQL with cast syntax
            sqlx_data::compile_time_only! {
                let _ = sqlx::query_as!(#struct_type, #sql, #(#param_names),*);
            }

            // Runtime execution - uses cleaned SQL without cast syntax
            sqlx::query_as::<_, #struct_type>(#cleaned_sql)
                #(#bind_calls)*
                #fetch_call
        })
    }
}

/// Generates paginated queries for Page<T> return types
pub struct PaginatedGenerator;

impl PaginatedGenerator {
    pub fn generate(context: &GenerationContext) -> syn::Result<TokenStream> {
        // Get the actual inner type (User from Serial<User>)
        if context.method.is_tuple_type() {
            Self::generate_tuple_pagination(context)
        } else {
            Self::generate_struct_pagination(context)
        }
    }

    /// Generate import for concrete parameter types (not impl types)
    fn generate_param_import(param_type: &syn::Type) -> TokenStream {
        match param_type {
            syn::Type::ImplTrait { .. } => {
                // impl IntoParams - no import needed (trait already in scope)
                quote! {}
            }
            syn::Type::Path(_) => {
                // Concrete type like SerialParams - needs IntoParams trait import
                quote! { use sqlx_data::IntoParams; }
            }
            _ => {
                // Fallback - no import
                quote! {}
            }
        }
    }

    fn generate_struct_pagination(context: &GenerationContext) -> syn::Result<TokenStream> {
        let method = context.method;

        // Get the actual inner type (User from Serial<User>)
        let inner_type = method.get_return_inner_type();
        let param_names = &context.param_names;
        let pool_expr = &context.pool_expr;
        let sql = &method.sql_content;

        // Extract pagination type name directly
        let pagination_variant = method.return_ok_type_name().ok_or_else(|| {
            error("Expected paginated return type (Serial<T>, Slice<T>, or Cursor<T>)")
        })?;

        let pagination_type = format_ident!("{}", pagination_variant);

        // Find PageRequest parameter
        let page_request_param = method.parameters.iter().find(|p| p.is_dynamic_param);

        match page_request_param {
            Some(parameter) => {
                let param_name = format_ident!("{}", parameter.name);
                let param_import = Self::generate_param_import(&parameter.type_);

                // Prepare SQL construction logic with cast syntax cleaning
                let cleaned_sql = clean_sqlx_cast_syntax_for_runtime(sql);

                // Generate initial binds from function parameters
                let initial_binds = MethodSignatureGenerator::generate_initial_binds(method);

                // Generate response creation logic based on pagination type
                let create_response = match pagination_variant.as_str() {
                    pagination::SERIAL => quote! {
                        // Generate count SQL from the Statement
                        let count_sql = sqlx_data::build_count_query_from_sql(&sql)?;
                        
                        // Build args from same bind values
                        let count_args = make_args()?;
                        let total_elements = sqlx::query_scalar_with(&*count_sql, count_args)
                            .fetch_one(#pool_expr)
                            .await?;

                        Ok(sqlx_data::#pagination_type::new(data, &params, total_elements))
                    },
                    pagination::SLICE => quote! {
                        // Slice has optional count - inverted if logic
                        let total_elements = if !params.is_disable_total_count() {
                            // Generate count SQL from the Statement
                            let count_sql = sqlx_data::build_count_query_from_sql(&sql)?;

                            // Build args from same bind values
                            sqlx::query_scalar_with(&*count_sql, make_args()?)
                                .fetch_one(#pool_expr)
                                .await?
                        } else {
                            0
                        };

                        Ok(sqlx_data::#pagination_type::new(data, &params, total_elements))
                    },
                    pagination::CURSOR => quote! {
                        sqlx_data::#pagination_type::new(data, &params)
                            .map_err(|e| sqlx_data::Error::Decode(e.to_string().into()))
                    },
                    _ => quote! {
                        // Fallback for unknown pagination types - inverted if logic
                        let total_elements = if !params.is_disable_total_count() {
                            // Generate count SQL from the Statement
                            let count_sql = sqlx_data::build_count_query_from_sql(&sql)?;

                            // Build args from same bind values
                            sqlx::query_scalar_with(&*count_sql, make_args()?)
                                .fetch_one(#pool_expr)
                                .await?
                        } else {
                            0
                        };

                        sqlx_data::#pagination_type::new(data, &params, total_elements)
                    },
                };

                // Generate compile-time validation only for checked queries
                let compile_validation = if !method.is_unchecked {
                    quote! {
                        // SQL validation (compile-time only, never executes)
                        sqlx_data::compile_time_only!(let _ = sqlx::query_as!(#inner_type, #sql, #(#param_names),*));
                    }
                } else {
                    quote! {} // Skip validation for unchecked queries
                };

                Ok(quote_spanned! { method.method_span() =>
                    #param_import

                    // Convert to Params for unified handling
                    let params = #param_name.into_params();

                    // Runtime validation of parameters
                    sqlx_data::validate_fields(&params)?;

                    #compile_validation

                    // Create initial_binds from function parameters
                    let initial_binds = #initial_binds;

                    // Build dynamic SQL with filters, search, and pagination
                    let built_sql = sqlx_data::build_dynamic_sql(
                        #cleaned_sql,
                        &params,
                        initial_binds
                    )?;

                    // Execute data query using Statement
                    let sql = built_sql.sql.as_ref();
                    let make_args = params.build_arguments(&built_sql.bind_values);
                    let data = sqlx::query_as_with::<_, #inner_type, _>(&sql, make_args()?)
                        .fetch_all(#pool_expr)
                        .await?;

                    // Create response based on pagination type
                    #create_response
                })
            }
            None => Err(method_error(
                method,
                "Page<T> return type requires a dynamic parameter (impl IntoParams, Params, PaginationParams, etc.)",
            )),
        }
    }

    fn generate_tuple_pagination(context: &GenerationContext) -> syn::Result<TokenStream> {
        let method = context.method;
        let param_names = &context.param_names;
        let pool_expr = &context.pool_expr;
        let sql = &method.sql_content;

        // Get the actual inner type (tuple from Serial<(i32, String)>)
        // Extract pagination type name directly
        let pagination_variant = method.return_ok_type_name().ok_or_else(|| {
            error("Expected paginated return type (Serial<T>, Slice<T>, or Cursor<T>)")
        })?;

        let pagination_type = format_ident!("{}", pagination_variant);

        // Find PageRequest parameter
        let page_request_param = method.parameters.iter().find(|p| p.is_dynamic_param);

        // Count query will be generated from Statement at runtime

        let tuple_types = method.parse_tuple_types()?;
        let inferred = sqlx_data_parser::infer_columns_from_stmt(
            method
                .statement
                .as_ref()
                .ok_or_else(|| error("SQL statement not available"))?,
        )
        .map_err(core_error)?;

        if inferred.columns.len() != tuple_types.len() {
            return Err(error(format!(
                "Tuple has {} elements but SQL query returns {} columns",
                tuple_types.len(),
                inferred.columns.len()
            )));
        }

        let struct_fields =
            TupleConversionGenerator::generate_struct_fields(&inferred.columns, &tuple_types);
        let tuple_conversion_row =
            TupleConversionGenerator::generate_conversion_row(&inferred.columns, &tuple_types);

        match page_request_param {
            Some(parameter) => {
                let params_ident = format_ident!("{}", parameter.name);
                let param_import = Self::generate_param_import(&parameter.type_);

                // Prepare SQL construction logic with cast syntax cleaning
                let cleaned_sql = clean_sqlx_cast_syntax_for_runtime(sql);

                // Generate initial binds from function parameters
                let initial_binds = MethodSignatureGenerator::generate_initial_binds(method);

                // Generate response creation logic based on pagination type (compile-time optimized)
                let create_response = match pagination_variant.as_str() {
                    pagination::SERIAL => quote! {
                        // Generate count SQL from the Statement
                        let count_sql = sqlx_data::build_count_query_from_sql(&sql)?;

                        // Serial always needs count - build args from same bind values
                        let total_elements = sqlx::query_scalar_with(&*count_sql, make_args()?)
                            .fetch_one(#pool_expr)
                            .await?;

                        Ok(sqlx_data::#pagination_type::new(data, &params, total_elements))
                    },
                    pagination::SLICE => quote! {
                        // Slice has optional count - inverted if logic
                        let total_elements = if !params.is_disable_total_count() {
                            // Generate count SQL from the Statement
                            let count_sql = sqlx_data::build_count_query_from_sql(&sql)?;

                            // Build args from same bind values
                            sqlx::query_scalar_with(&*count_sql, make_args()?)
                                .fetch_one(#pool_expr)
                                .await?
                        } else {
                            0
                        };

                        Ok(sqlx_data::#pagination_type::new(data, &params, total_elements))
                    },
                    pagination::CURSOR => quote! {
                        // Cursor never needs count - no total_elements parameter
                        sqlx_data::#pagination_type::new(data, &params)
                            .map_err(|e| sqlx_data::Error::Decode(e.to_string().into()))
                    },
                    _ => quote! {
                        // Fallback for unknown pagination types - inverted if logic
                        let total_elements = if !params.is_disable_total_count() {
                            // Generate count SQL from the Statement
                            let count_sql = sqlx_data::build_count_query_from_sql(&sql)?;

                            // Build args from same bind values
                            sqlx::query_scalar_with(&*count_sql, make_args()?)
                                .fetch_one(#pool_expr)
                                .await?
                        } else {
                            0
                        };

                        sqlx_data::#pagination_type::new(data, &params, total_elements)
                    },
                };

                // Generate compile-time validation only for checked queries
                let compile_validation = if !method.is_unchecked {
                    quote! {
                        // SQL validation (compile-time only, never executes)
                        sqlx_data::compile_time_only!(let _ = sqlx::query_as!(QueryTuple, #sql, #(#param_names),*));
                    }
                } else {
                    quote! {} // Skip validation for unchecked queries
                };

                Ok(quote_spanned! { method.method_span() =>
                    #param_import

                    // Convert to Params for unified handling
                    let params = #params_ident.into_params();

                    // Runtime validation of parameters
                    sqlx_data::validate_fields(&params)?;

                    #[derive(sqlx::FromRow)]
                    struct QueryTuple {
                        #(#struct_fields),*
                    }

                    #compile_validation

                    // Create initial_binds from function parameters
                    let initial_binds = #initial_binds;

                    // Build dynamic SQL with filters, search, and pagination
                    let built_sql = sqlx_data::build_dynamic_sql(
                        #cleaned_sql,
                        &params,
                        initial_binds
                    )?;

                    // Execute data query using Statement
                    let sql = built_sql.sql.as_ref();
                    let make_args = params.build_arguments(&built_sql.bind_values);
                    let rows = sqlx::query_as_with::<_, QueryTuple, _>(&sql, make_args()?)
                        .fetch_all(#pool_expr)
                        .await?;

                    let data: Vec<_> = rows
                        .into_iter()
                        .map(|row| (#(#tuple_conversion_row),*))
                        .collect();

                    // Create response based on pagination type
                    #create_response
                })
            }
            None => Err(method_error(
                method,
                "Page<T> return type requires a dynamic parameter (impl IntoParams, Params, PaginationParams, etc.)",
            )),
        }
    }
}

/// Helper generators for specific code patterns
pub struct TupleConversionGenerator;
pub struct DocumentationGenerator;

impl TupleConversionGenerator {
    pub fn generate_conversion_row(
        columns: &[String],
        tuple_types: &[syn::Type],
    ) -> Vec<TokenStream> {
        columns
            .iter()
            .enumerate()
            .map(|(i, column_name)| {
                let field_name = format_ident!("{}", extract_column_name(column_name));
                let target_type = &tuple_types[i];

                // Check if the column has explicit SQLx type casting
                if has_explicit_sqlx_type(column_name) {
                    // With explicit casting, SQLx already provides the correct type
                    // No additional conversion needed - just return the field value
                    quote! { row.#field_name }
                } else {
                    // No explicit casting - use the regular conversion logic
                    let (is_option, actual_target_type) = if let Some(inner_type) =
                        TypeCastingAnalyzer::extract_option_type(target_type)
                    {
                        (true, inner_type)
                    } else {
                        (false, target_type.clone())
                    };

                    // Use the shared conversion logic
                    generate_conversion_expr(
                        quote! { row.#field_name },
                        &actual_target_type,
                        is_option,
                        true, // is_tuple: this is a tuple element
                    )
                }
            })
            .collect()
    }

    pub fn generate_struct_fields(
        columns: &[String],
        tuple_types: &[syn::Type],
    ) -> Vec<TokenStream> {
        columns
            .iter()
            .enumerate()
            .map(|(i, column_name)| {
                let field_name = format_ident!("{}", extract_column_name(column_name));
                let target_type = &tuple_types[i];

                // Check if the column has explicit SQLx type casting
                let struct_field_type = if has_explicit_sqlx_type(column_name) {
                    // Use the explicit type from SQLx cast syntax
                    if let Some(explicit_type) = extract_sqlx_explicit_type(column_name) {
                        // For Option<T> target types, wrap the explicit type in Option
                        if TypeCastingAnalyzer::extract_option_type(target_type).is_some() {
                            syn::parse_quote! { Option<#explicit_type> }
                        } else {
                            explicit_type
                        }
                    } else {
                        // Fallback to native type if parsing failed
                        if let Some(inner_type) =
                            TypeCastingAnalyzer::extract_option_type(target_type)
                        {
                            let inner_native = TypeCastingAnalyzer::native_type(&inner_type);
                            syn::parse_quote! { Option<#inner_native> }
                        } else {
                            TypeCastingAnalyzer::native_type(target_type)
                        }
                    }
                } else {
                    // No explicit casting - use native database types as before
                    if let Some(inner_type) = TypeCastingAnalyzer::extract_option_type(target_type)
                    {
                        let inner_native = TypeCastingAnalyzer::native_type(&inner_type);
                        syn::parse_quote! { Option<#inner_native> }
                    } else {
                        TypeCastingAnalyzer::native_type(target_type)
                    }
                };

                quote! { pub #field_name: #struct_field_type }
            })
            .collect()
    }

    pub fn generate_conversion_code(
        conversions: &[TokenStream],
        fetch_method: &FetchMethod,
    ) -> TokenStream {
        match fetch_method {
            FetchMethod::FetchAll => {
                quote! {
                    Ok(rows.into_iter()
                        .map(|row| (#(#conversions),*))
                        .collect())
                }
            }
            FetchMethod::FetchOne => {
                quote! {
                    Ok((#(#conversions),*))
                }
            }
            FetchMethod::FetchOptional => {
                quote! {
                    Ok(row_option.map(|row| (#(#conversions),*)))
                }
            }
            FetchMethod::Execute => {
                quote! {
                    Ok(()) // Execute doesn't return data
                }
            }
            FetchMethod::Fetch => {
                quote! {
                    // Stream processing - not used in tuple conversion
                    Ok(())
                }
            }
        }
    }

    pub fn generate_for_fetch(fetch_method: &FetchMethod) -> TokenStream {
        match fetch_method {
            FetchMethod::FetchAll => quote! { rows },
            FetchMethod::FetchOne => quote! { row },
            FetchMethod::FetchOptional => quote! { row_option },
            FetchMethod::Execute => quote! { result },
            FetchMethod::Fetch => quote! { stream },
        }
    }
}

/// Generate conversion expression for a value (reusable for scalar and tuple elements)
/// Generate conversion expression for a value (reusable for scalar and tuple elements)
fn generate_conversion_expr(
    value_expr: TokenStream,
    target_type: &syn::Type,
    is_option: bool,
    is_tuple: bool,
) -> TokenStream {
    if !TypeCastingAnalyzer::needs_casting(target_type) {
        return value_expr;
    }

    match is_option {
        true => option_cast_expr(value_expr, target_type, is_tuple),
        false => cast_expr(value_expr, target_type),
    }
}


fn cast_expr(value: TokenStream, target: &syn::Type) -> TokenStream {
    if is_bool(target) {
        quote! {
            #value != 0
        }
    } else {
        quote! {
            #value as #target
        }
    }
}

fn option_cast_expr(value: TokenStream, target: &syn::Type, is_tuple: bool) -> TokenStream {
    let value = if is_tuple {
        quote! { #value }
    } else {
        quote! { #value.flatten() }
    };

    if is_bool(target) {
        quote! {
            #value.map(|v| v != 0)
        }
    } else {
        quote! {
            #value.map(|v| v as #target)
        }
    }
}

fn is_bool(ty: &syn::Type) -> bool {
    matches!(ty, syn::Type::Path(p) if p.path.is_ident("bool"))
}

impl DocumentationGenerator {
    pub fn format_generated_code(token_stream: &TokenStream) -> String {
        match syn::parse2::<syn::ItemFn>(token_stream.clone()) {
            Ok(parsed_fn) => {
                // Keep the function's attributes when formatting for documentation
                prettyplease::unparse(&syn::File {
                    shebang: None,
                    attrs: vec![], // File-level attributes, not function attributes
                    items: vec![syn::Item::Fn(parsed_fn)], // Function keeps its own attributes
                })
            }
            Err(_) => token_stream.to_string(),
        }
    }

    pub fn generate_doc_comment(query_code: &str) -> String {
        format!("Generated by #[dml] macro:\n\n```rust\n{}\n```", query_code)
    }
}

/// Helper function for creating errors with call_site span
fn error(message: impl Into<String>) -> syn::Error {
    syn::Error::new(proc_macro2::Span::call_site(), message.into())
}

/// Helper function for creating errors with method span
fn method_error(method: &DmlMethod, message: impl Into<String>) -> syn::Error {
    syn::Error::new(method.method_span(), message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_column_name() {
        // Test SQLx cast syntax extraction
        assert_eq!(extract_column_name("email_length: i64"), "email_length");
        assert_eq!(extract_column_name("name: String"), "name");
        assert_eq!(extract_column_name("age: u8"), "age");
        assert_eq!(extract_column_name("birth_year: u16"), "birth_year");

        // Test force cast syntax with ! (should remove the !)
        assert_eq!(extract_column_name("email_length!: i64"), "email_length");
        assert_eq!(extract_column_name("name!: String"), "name");

        // Test normal column names without cast
        assert_eq!(extract_column_name("email_length"), "email_length");
        assert_eq!(extract_column_name("name"), "name");
        assert_eq!(extract_column_name("age"), "age");

        // Test with whitespace
        assert_eq!(extract_column_name("email_length : i64"), "email_length");
        assert_eq!(extract_column_name(" name : String "), "name");
    }

    #[test]
    fn test_clean_sqlx_cast_syntax() {
        // Test basic type overrides with single quotes
        assert_eq!(
            clean_sqlx_cast_syntax_for_runtime("SELECT id as 'id: i64', name as 'name: String'"),
            "SELECT id as id, name as name"
        );

        // Test basic type overrides with double quotes
        assert_eq!(
            clean_sqlx_cast_syntax_for_runtime(
                "SELECT id as \"id: i64\", name as \"name: String\""
            ),
            "SELECT id as id, name as name"
        );

        // Test basic type overrides with backticks
        assert_eq!(
            clean_sqlx_cast_syntax_for_runtime("SELECT id as `id: i64`, name as `name: String`"),
            "SELECT id as id, name as name"
        );

        // Test forced not-null with type override (foo!: T) - ! should be removed as it's SQLx-only
        assert_eq!(
            clean_sqlx_cast_syntax_for_runtime("SELECT id as 'id!: i64', name as 'name!: String'"),
            "SELECT id as id, name as name"
        );

        // Test forced nullable with type override (foo?: T) - ? should be removed as it's SQLx-only
        assert_eq!(
            clean_sqlx_cast_syntax_for_runtime(
                "SELECT id as 'id?: Option<i64>', name as 'name?: Option<String>'"
            ),
            "SELECT id as id, name as name"
        );

        // Test mixed quote types in same query
        assert_eq!(
            clean_sqlx_cast_syntax_for_runtime(
                "SELECT id as 'id: i64', name as \"name: String\", age as `age: u32`"
            ),
            "SELECT id as id, name as name, age as age"
        );

        // Test preserve normal aliases (without colons) - should NOT be changed
        assert_eq!(
            clean_sqlx_cast_syntax_for_runtime(
                "SELECT COUNT(*) as 'total_count', AVG(age) as 'average_age'"
            ),
            "SELECT COUNT(*) as 'total_count', AVG(age) as 'average_age'"
        );

        // Test mixed normal aliases and cast syntax
        assert_eq!(
            clean_sqlx_cast_syntax_for_runtime(
                "SELECT id as 'id: i64', COUNT(*) as 'total', name as \"name: String\""
            ),
            "SELECT id as id, COUNT(*) as 'total', name as name"
        );

        // Test spaces around colons
        assert_eq!(
            clean_sqlx_cast_syntax_for_runtime(
                "SELECT id as 'id   :   i64', name as 'name:String'"
            ),
            "SELECT id as id, name as name"
        );

        // Test empty input
        assert_eq!(clean_sqlx_cast_syntax_for_runtime(""), "");

        // Real-world cases from actual SQLx usage
        assert_eq!(
            clean_sqlx_cast_syntax_for_runtime("SELECT id as 'id: Id!', name as 'name: String'"),
            "SELECT id as id, name as name"
        );

        assert_eq!(
            clean_sqlx_cast_syntax_for_runtime(
                "SELECT created_at as 'created_at?: timestamptz', status as 'status!: StatusEnum'"
            ),
            "SELECT created_at as created_at, status as status"
        );

        assert_eq!(
            clean_sqlx_cast_syntax_for_runtime(
                "SELECT user_id as \"user_id!: Uuid\", score as \"score?: Option<f64>\""
            ),
            "SELECT user_id as user_id, score as score"
        );

        // Complex real case from production_pagination_casting.rs
        assert_eq!(
            clean_sqlx_cast_syntax_for_runtime(
                "SELECT id as 'id!: Id', name, email, age as 'age: u8', birth_year as 'birth_year: u16' FROM users WHERE age BETWEEN $1 AND $2"
            ),
            "SELECT id as id, name, email, age as age, birth_year as birth_year FROM users WHERE age BETWEEN $1 AND $2"
        );
    }
}
