use crate::dml::{DmlMethod, DmlParameter};
use crate::type_analyzer::TypeAnalyzer;
use crate::type_system::{QueryType, ReturnType};
use quote::format_ident;
use sqlx_data_parser::SqlStatementType;
/// Comprehensive testing framework for sqlx-data macros
///
/// This module provides structured testing utilities for validating
/// macro functionality, type analysis, and code generation.
use syn::parse_quote;

/// Test framework for macro functionality
pub struct MacroTestFramework;

impl MacroTestFramework {
    /// Run all comprehensive tests
    pub fn run_all_tests() -> Result<(), TestError> {
        Self::test_type_analysis()?;
        Self::test_query_strategies()?;
        Self::test_code_generation()?;
        Self::test_edge_cases()?;
        println!("✅ All macro tests passed!");
        Ok(())
    }

    /// Test type analysis functionality
    fn test_type_analysis() -> Result<(), TestError> {
        println!("🧪 Testing type analysis...");

        // Test scalar types
        TypeAnalysisTests::test_scalar_types()?;

        // Test Result type alias
        TypeAnalysisTests::test_result_type_alias()?;

        // Test complex types
        TypeAnalysisTests::test_complex_types()?;

        // Test Option types
        TypeAnalysisTests::test_option_types()?;

        // Test Vec types
        TypeAnalysisTests::test_vec_types()?;

        // Test tuple types
        TypeAnalysisTests::test_tuple_types()?;

        println!("✅ Type analysis tests passed!");
        Ok(())
    }

    /// Test query strategy determination
    fn test_query_strategies() -> Result<(), TestError> {
        println!("🧪 Testing query strategies...");

        QueryStrategyTests::test_scalar_strategies()?;
        QueryStrategyTests::test_struct_strategies()?;
        QueryStrategyTests::test_tuple_strategies()?;
        QueryStrategyTests::test_collection_strategies()?;

        println!("✅ Query strategy tests passed!");
        Ok(())
    }

    /// Test code generation
    fn test_code_generation() -> Result<(), TestError> {
        println!("🧪 Testing code generation...");

        CodeGenerationTests::test_basic_scalar_generation()?;
        CodeGenerationTests::test_struct_generation()?;
        CodeGenerationTests::test_tuple_generation()?;
        CodeGenerationTests::test_parameter_handling()?;

        println!("✅ Code generation tests passed!");
        Ok(())
    }

    /// Test edge cases and error conditions
    fn test_edge_cases() -> Result<(), TestError> {
        println!("🧪 Testing edge cases...");

        EdgeCaseTests::test_empty_parameters()?;
        EdgeCaseTests::test_complex_nested_types()?;
        EdgeCaseTests::test_invalid_sql_handling()?;
        EdgeCaseTests::test_type_mismatch_detection()?;

        println!("✅ Edge case tests passed!");
        Ok(())
    }
}

/// Type analysis specific tests
pub struct TypeAnalysisTests;

impl TypeAnalysisTests {
    pub fn test_scalar_types() -> Result<(), TestError> {
        // Test primitive scalar types
        let test_cases = [
            (
                "i32",
                ReturnType::Scalar {
                    name: format_ident!("i32"),
                },
            ),
            (
                "i64",
                ReturnType::Scalar {
                    name: format_ident!("i64"),
                },
            ),
            (
                "String",
                ReturnType::Scalar {
                    name: format_ident!("String"),
                },
            ),
            (
                "bool",
                ReturnType::Scalar {
                    name: format_ident!("bool"),
                },
            ),
            (
                "f64",
                ReturnType::Scalar {
                    name: format_ident!("f64"),
                },
            ),
        ];

        for (type_str, expected) in test_cases {
            let ty: syn::Type = syn::parse_str(type_str).map_err(|e| {
                TestError::ParseError(format!("Failed to parse {}: {}", type_str, e))
            })?;

            let analyzed = TypeAnalyzer::analyze_type(&ty).map_err(|e| {
                TestError::ParseError(format!("Failed to analyze {}: {}", type_str, e))
            })?;

            if !matches_return_type(&analyzed, &expected) {
                return Err(TestError::AssertionError(format!(
                    "Type analysis mismatch for {}: expected {:?}, got {:?}",
                    type_str, expected, analyzed
                )));
            }
        }

        Ok(())
    }

    pub fn test_result_type_alias() -> Result<(), TestError> {
        // Test Result<T> type alias (single parameter)
        let ty: syn::Type = parse_quote!(Result<i32>);
        let analyzed = TypeAnalyzer::analyze_type(&ty)
            .map_err(|e| TestError::ParseError(format!("Failed to analyze Result<i32>: {}", e)))?;

        match analyzed {
            ReturnType::Result { ok_type, err_type } => {
                match ok_type.as_ref() {
                    ReturnType::Scalar { name } if name.to_string() == "i32" => (),
                    _ => {
                        return Err(TestError::AssertionError(
                            "Result<i32> should have i32 as ok_type".to_string(),
                        ));
                    }
                }
                match err_type.as_ref() {
                    ReturnType::Unknown { name } if name == "sqlx_data::Error" => (),
                    _ => {
                        return Err(TestError::AssertionError(format!(
                            "Result<T> should have sqlx_data::Error as err_type, but got: {:?}",
                            err_type
                        )));
                    }
                }
            }
            _ => {
                return Err(TestError::AssertionError(
                    "Result<i32> should be analyzed as Result type".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub fn test_complex_types() -> Result<(), TestError> {
        // Test Result<Option<Vec<User>>>
        let ty: syn::Type = parse_quote!(Result<Option<Vec<User>>>);
        let analyzed = TypeAnalyzer::analyze_type(&ty)
            .map_err(|e| TestError::ParseError(format!("Failed to analyze complex type: {}", e)))?;

        match analyzed {
            ReturnType::Result { ok_type, .. } => match ok_type.as_ref() {
                ReturnType::Option { inner_type } => match inner_type.as_ref() {
                    ReturnType::Vec { element_type } => match element_type.as_ref() {
                        ReturnType::Struct { name } if name.to_string() == "User" => (),
                        _ => {
                            return Err(TestError::AssertionError(
                                "Expected User struct in Vec".to_string(),
                            ));
                        }
                    },
                    _ => {
                        return Err(TestError::AssertionError(
                            "Expected Vec in Option".to_string(),
                        ));
                    }
                },
                _ => {
                    return Err(TestError::AssertionError(
                        "Expected Option in Result".to_string(),
                    ));
                }
            },
            _ => {
                return Err(TestError::AssertionError(
                    "Expected Result type".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub fn test_option_types() -> Result<(), TestError> {
        let ty: syn::Type = parse_quote!(Option<String>);
        let analyzed = TypeAnalyzer::analyze_type(&ty)
            .map_err(|e| TestError::ParseError(format!("Failed to analyze Option type: {}", e)))?;

        match analyzed {
            ReturnType::Option { inner_type } => match inner_type.as_ref() {
                ReturnType::Scalar { name } if name.to_string() == "String" => (),
                _ => {
                    return Err(TestError::AssertionError(
                        "Option<String> should contain String scalar".to_string(),
                    ));
                }
            },
            _ => {
                return Err(TestError::AssertionError(
                    "Option<String> should be analyzed as Option type".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub fn test_vec_types() -> Result<(), TestError> {
        let ty: syn::Type = parse_quote!(Vec<User>);
        let analyzed = TypeAnalyzer::analyze_type(&ty)
            .map_err(|e| TestError::ParseError(format!("Failed to analyze Vec type: {}", e)))?;

        match analyzed {
            ReturnType::Vec { element_type } => match element_type.as_ref() {
                ReturnType::Struct { name } if name.to_string() == "User" => (),
                _ => {
                    return Err(TestError::AssertionError(
                        "Vec<User> should contain User struct".to_string(),
                    ));
                }
            },
            _ => {
                return Err(TestError::AssertionError(
                    "Vec<User> should be analyzed as Vec type".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub fn test_tuple_types() -> Result<(), TestError> {
        let ty: syn::Type = parse_quote!((i32, String, bool));
        let analyzed = TypeAnalyzer::analyze_type(&ty)
            .map_err(|e| TestError::ParseError(format!("Failed to analyze Tuple type: {}", e)))?;

        match analyzed {
            ReturnType::Tuple { elements } => {
                if elements.len() != 3 {
                    return Err(TestError::AssertionError(
                        "Tuple should have 3 elements".to_string(),
                    ));
                }

                // Check each element
                match (&elements[0], &elements[1], &elements[2]) {
                    (
                        ReturnType::Scalar { name: name1 },
                        ReturnType::Scalar { name: name2 },
                        ReturnType::Scalar { name: name3 },
                    ) if name1 == "i32" && name2 == "String" && name3 == "bool" => (),
                    _ => {
                        return Err(TestError::AssertionError(
                            "Tuple elements don't match expected types".to_string(),
                        ));
                    }
                }
            }
            _ => {
                return Err(TestError::AssertionError(
                    "(i32, String, bool) should be analyzed as Tuple type".to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// Query strategy specific tests
pub struct QueryStrategyTests;

impl QueryStrategyTests {
    pub fn test_scalar_strategies() -> Result<(), TestError> {
        // Test scalar types result in QueryScalar
        let ty: syn::Type = parse_quote!(Result<i32>);
        let analyzed = TypeAnalyzer::analyze_type(&ty)
            .map_err(|e| TestError::ParseError(format!("Failed to analyze type: {}", e)))?;
        let strategy = TypeAnalyzer::determine_query_strategy(&analyzed);

        match strategy {
            Ok(QueryType::QueryScalar) => {
                // Test passed - scalar types correctly result in QueryScalar
            }
            _ => {
                return Err(TestError::AssertionError(
                    "Result<i32> should use QueryScalar strategy".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub fn test_struct_strategies() -> Result<(), TestError> {
        let ty: syn::Type = parse_quote!(Result<User>);
        let analyzed = TypeAnalyzer::analyze_type(&ty)
            .map_err(|e| TestError::ParseError(format!("Failed to analyze type: {}", e)))?;
        let strategy = TypeAnalyzer::determine_query_strategy(&analyzed);

        match strategy {
            Ok(QueryType::QueryAs) => {
                // Test passed - struct types correctly result in QueryAs
            }
            _ => {
                return Err(TestError::AssertionError(
                    "Result<User> should use QueryAs strategy".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub fn test_tuple_strategies() -> Result<(), TestError> {
        let ty: syn::Type = parse_quote!(Result<(i32, String)>);
        let analyzed = TypeAnalyzer::analyze_type(&ty)
            .map_err(|e| TestError::ParseError(format!("Failed to analyze type: {}", e)))?;
        let strategy = TypeAnalyzer::determine_query_strategy(&analyzed);

        match strategy {
            Ok(QueryType::QueryAs) => {
                // Test passed - tuple types correctly result in QueryAs
            }
            _ => {
                return Err(TestError::AssertionError(
                    "Result<(i32, String)> should use QueryAs strategy".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub fn test_collection_strategies() -> Result<(), TestError> {
        // Test Vec<T> uses fetch_all
        let ty: syn::Type = parse_quote!(Result<Vec<User>>);
        let analyzed = TypeAnalyzer::analyze_type(&ty)
            .map_err(|e| TestError::ParseError(format!("Failed to analyze type: {}", e)))?;
        let strategy = TypeAnalyzer::determine_query_strategy(&analyzed);

        match strategy {
            Ok(QueryType::QueryAs) => {
                // Test passed - Vec<T> correctly results in QueryAs
            }
            _ => {
                return Err(TestError::AssertionError(
                    "Result<Vec<User>> should use QueryAs strategy with fetch_all".to_string(),
                ));
            }
        }

        // Test Option<T> uses fetch_optional
        let ty: syn::Type = parse_quote!(Result<Option<User>>);
        let analyzed = TypeAnalyzer::analyze_type(&ty)
            .map_err(|e| TestError::ParseError(format!("Failed to analyze type: {}", e)))?;
        let strategy = TypeAnalyzer::determine_query_strategy(&analyzed);

        match strategy {
            Ok(QueryType::QueryAs) => {
                // Test passed - Option<T> correctly results in QueryAs
            }
            _ => {
                return Err(TestError::AssertionError(
                    "Result<Option<User>> should use QueryAs strategy with fetch_optional"
                        .to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// Code generation specific tests
pub struct CodeGenerationTests;

impl CodeGenerationTests {
    pub fn test_basic_scalar_generation() -> Result<(), TestError> {
        // Test basic scalar DML method generation
        let method = create_test_dml_method(
            "get_count",
            "SELECT COUNT(*) as count FROM users",
            vec![],
            parse_quote!(Result<i64>),
            SqlStatementType::Select,
        );

        let result = crate::code_generator::CodeGenerator::generate_dml_methods(&method);

        match result {
            Ok(generated) => {
                let code_str = generated.to_string();
                // Basic checks for generated code
                if !code_str.contains("get_count_query") {
                    return Err(TestError::AssertionError(
                        "Generated code should contain query method".to_string(),
                    ));
                }
                if !code_str.contains("get_count") {
                    return Err(TestError::AssertionError(
                        "Generated code should contain main method".to_string(),
                    ));
                }
            }
            Err(e) => {
                return Err(TestError::CodeGenerationError(format!(
                    "Failed to generate scalar method: {}",
                    e
                )));
            }
        }

        Ok(())
    }

    pub fn test_struct_generation() -> Result<(), TestError> {
        let method = create_test_dml_method(
            "find_user",
            "SELECT id, name, email FROM users WHERE id = $1",
            vec![DmlParameter {
                name: "id".to_string(),
                type_: parse_quote!(i64),
                is_pool: false,
                is_dynamic_param: false,
                is_generic: false,
            }],
            parse_quote!(Result<User>),
            SqlStatementType::Select,
        );

        let result = crate::code_generator::CodeGenerator::generate_dml_methods(&method);

        match result {
            Ok(generated) => {
                let code_str = generated.to_string();
                if !code_str.contains("find_user_query") || !code_str.contains("find_user") {
                    return Err(TestError::AssertionError(
                        "Generated code missing expected methods".to_string(),
                    ));
                }
            }
            Err(e) => {
                return Err(TestError::CodeGenerationError(format!(
                    "Failed to generate struct method: {}",
                    e
                )));
            }
        }

        Ok(())
    }

    pub fn test_tuple_generation() -> Result<(), TestError> {
        let method = create_test_dml_method(
            "get_stats",
            "SELECT COUNT(*) as count, AVG(age) as avg_age FROM users",
            vec![],
            parse_quote!(Result<(i64, f64)>),
            SqlStatementType::Select,
        );

        let result = crate::code_generator::CodeGenerator::generate_dml_methods(&method);

        match result {
            Ok(generated) => {
                let code_str = generated.to_string();
                if !code_str.contains("get_stats_query") {
                    return Err(TestError::AssertionError(
                        "Generated code should contain query method for tuple".to_string(),
                    ));
                }
            }
            Err(e) => {
                return Err(TestError::CodeGenerationError(format!(
                    "Failed to generate tuple method: {}",
                    e
                )));
            }
        }

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub fn test_tuple_casting() -> Result<(), TestError> {
        let method = create_test_dml_method(
            "avg_stats",
            "SELECT birth_year, AVG(age) as avg_age FROM users WHERE birth_year IS NOT NULL GROUP BY birth_year HAVING AVG(age) >$1",
            vec![],
            parse_quote!(Result<Vec<(Option<u16>, Option<f32>)>>),
            SqlStatementType::Select,
        );

        let result = crate::code_generator::CodeGenerator::generate_dml_methods(&method);

        match result {
            Ok(generated) => {
                let code_str = generated.to_string();
                // Check that f64 -> f32 casting is generated correctly (SQLite returns f64 for AVG)
                if !code_str.contains("as f32") {
                    return Err(TestError::AssertionError(
                        "Generated code should contain f32 casting".to_string(),
                    ));
                }
                // Check that i64 -> u16 casting is present (SQLite uses i64 for all integers)
                if !code_str.contains("as u16") {
                    return Err(TestError::AssertionError(
                        "Generated code should contain u16 casting".to_string(),
                    ));
                }
            }
            Err(e) => {
                return Err(TestError::CodeGenerationError(format!(
                    "Failed to generate tuple casting: {}",
                    e
                )));
            }
        }

        Ok(())
    }

    pub fn test_parameter_handling() -> Result<(), TestError> {
        let method = create_test_dml_method(
            "find_by_age_range",
            "SELECT id, name, age FROM users WHERE age BETWEEN $1 AND $2",
            vec![
                DmlParameter {
                    name: "min_age".to_string(),
                    type_: parse_quote!(u8),
                    is_pool: false,
                    is_dynamic_param: false,
                    is_generic: false,
                },
                DmlParameter {
                    name: "max_age".to_string(),
                    type_: parse_quote!(u8),
                    is_pool: false,
                    is_dynamic_param: false,
                    is_generic: false,
                },
            ],
            parse_quote!(Result<Vec<User>>),
            SqlStatementType::Select,
        );

        let result = crate::code_generator::CodeGenerator::generate_dml_methods(&method);

        match result {
            Ok(generated) => {
                let code_str = generated.to_string();
                if !code_str.contains("min_age") || !code_str.contains("max_age") {
                    return Err(TestError::AssertionError(
                        "Generated code should contain parameter names".to_string(),
                    ));
                }
            }
            Err(e) => {
                return Err(TestError::CodeGenerationError(format!(
                    "Failed to generate method with parameters: {}",
                    e
                )));
            }
        }

        Ok(())
    }
}

/// Edge case specific tests
pub struct EdgeCaseTests;

impl EdgeCaseTests {
    pub fn test_empty_parameters() -> Result<(), TestError> {
        let method = create_test_dml_method(
            "get_all",
            "SELECT id, name, email FROM users",
            vec![],
            parse_quote!(Result<Vec<User>>),
            SqlStatementType::Select,
        );

        let result = crate::code_generator::CodeGenerator::generate_dml_methods(&method);

        if result.is_err() {
            return Err(TestError::CodeGenerationError(
                "Should handle empty parameters".to_string(),
            ));
        }

        Ok(())
    }

    pub fn test_complex_nested_types() -> Result<(), TestError> {
        let ty: syn::Type = parse_quote!(Result<Option<Vec<(i32, Option<String>)>>>);
        let analyzed = TypeAnalyzer::analyze_type(&ty).map_err(|e| {
            TestError::ParseError(format!("Failed to analyze complex nested type: {}", e))
        })?;

        // Should not panic or fail catastrophically
        let _strategy = TypeAnalyzer::determine_query_strategy(&analyzed);

        Ok(())
    }

    pub fn test_invalid_sql_handling() -> Result<(), TestError> {
        // This should be handled gracefully by the SQL parser
        let method = create_test_dml_method(
            "invalid_sql",
            "INVALID SQL SYNTAX",
            vec![],
            parse_quote!(Result<i32>),
            SqlStatementType::Select,
        );

        let result = crate::code_generator::CodeGenerator::generate_dml_methods(&method);

        // Should either succeed (if SQL parser is lenient) or provide meaningful error
        match result {
            Ok(_) => (), // SQL parser was lenient
            Err(e) => {
                // Should have meaningful error message
                if e.to_string().is_empty() {
                    return Err(TestError::AssertionError(
                        "Error message should not be empty".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    pub fn test_type_mismatch_detection() -> Result<(), TestError> {
        // This tests the type system's ability to handle edge cases
        let ty: syn::Type = parse_quote!(Result<UnknownType>);
        let analyzed = TypeAnalyzer::analyze_type(&ty)
            .map_err(|e| TestError::ParseError(format!("Failed to analyze unknown type: {}", e)))?;

        match analyzed {
            ReturnType::Result { ok_type, .. } => match ok_type.as_ref() {
                ReturnType::Struct { name } if name.to_string() == "UnknownType" => (),
                _ => {
                    return Err(TestError::AssertionError(
                        "Unknown type should be analyzed as struct".to_string(),
                    ));
                }
            },
            _ => {
                return Err(TestError::AssertionError(
                    "Should handle unknown types gracefully".to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// Helper functions for tests
fn create_test_dml_method(
    name: &str,
    sql: &str,
    parameters: Vec<DmlParameter>,
    return_type: syn::Type,
    kind: SqlStatementType,
) -> DmlMethod {
    use syn::{FnArg, Pat, PatIdent, PatType, Signature, TraitItemFn};

    // Create function signature
    let mut inputs = syn::punctuated::Punctuated::new();

    // Add self parameter
    inputs.push(FnArg::Receiver(syn::Receiver {
        attrs: vec![],
        reference: Some((syn::Token![&](proc_macro2::Span::call_site()), None)),
        mutability: None,
        self_token: syn::Token![self](proc_macro2::Span::call_site()),
        colon_token: None,
        ty: Box::new(syn::parse_quote! { Self }),
    }));

    // Add other parameters
    for param in &parameters {
        let pat = PatIdent {
            attrs: vec![],
            by_ref: None,
            mutability: None,
            ident: syn::Ident::new(&param.name, proc_macro2::Span::call_site()),
            subpat: None,
        };

        inputs.push(FnArg::Typed(PatType {
            attrs: vec![],
            pat: Box::new(Pat::Ident(pat)),
            colon_token: syn::Token![:](proc_macro2::Span::call_site()),
            ty: Box::new(param.type_.clone()),
        }));
    }

    let sig = Signature {
        constness: None,
        asyncness: Some(syn::Token![async](proc_macro2::Span::call_site())),
        unsafety: None,
        abi: None,
        fn_token: syn::Token![fn](proc_macro2::Span::call_site()),
        ident: syn::Ident::new(name, proc_macro2::Span::call_site()),
        generics: syn::Generics::default(),
        paren_token: syn::token::Paren::default(),
        inputs,
        variadic: None,
        output: syn::ReturnType::Type(
            syn::Token![->](proc_macro2::Span::call_site()),
            Box::new(return_type),
        ),
    };

    let trait_method = TraitItemFn {
        attrs: vec![],
        sig,
        default: None,
        semi_token: Some(syn::Token![;](proc_macro2::Span::call_site())),
    };

    DmlMethod {
        method: trait_method,
        sql_content: sql.to_string(),
        parameters,
        statement: sqlx_data_parser::parse_sql(sql).unwrap(),
        kind,
        is_json_query: false,
        is_multi_insert: false,
        is_unchecked: false,
        has_explicit_instrument: false,
        trait_instrument: false,
        return_info_cache: std::sync::OnceLock::new(),
    }
}

fn matches_return_type(actual: &ReturnType, expected: &ReturnType) -> bool {
    match (actual, expected) {
        (ReturnType::Scalar { name: n1 }, ReturnType::Scalar { name: n2 }) => n1 == n2,
        (ReturnType::Struct { name: n1 }, ReturnType::Struct { name: n2 }) => n1 == n2,
        _ => false, // Simplified comparison for basic tests
    }
}

/// Test error types
#[derive(Debug)]
pub enum TestError {
    ParseError(String),
    AssertionError(String),
    CodeGenerationError(String),
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            TestError::AssertionError(msg) => write!(f, "Assertion failed: {}", msg),
            TestError::CodeGenerationError(msg) => write!(f, "Code generation error: {}", msg),
        }
    }
}

impl std::error::Error for TestError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_comprehensive() {
        // Run all comprehensive tests
        match MacroTestFramework::run_all_tests() {
            Ok(()) => println!("🎉 All comprehensive tests passed!"),
            Err(e) => panic!("Test failed: {}", e),
        }
    }

    #[test]
    fn test_individual_type_analysis() {
        TypeAnalysisTests::test_scalar_types().unwrap();
        TypeAnalysisTests::test_result_type_alias().unwrap();
        TypeAnalysisTests::test_complex_types().unwrap();
    }

    #[test]
    fn test_individual_query_strategies() {
        QueryStrategyTests::test_scalar_strategies().unwrap();
        QueryStrategyTests::test_struct_strategies().unwrap();
        QueryStrategyTests::test_collection_strategies().unwrap();
    }

    #[test]
    fn test_individual_code_generation() {
        CodeGenerationTests::test_basic_scalar_generation().unwrap();
        CodeGenerationTests::test_struct_generation().unwrap();
        #[cfg(feature = "sqlite")]
        CodeGenerationTests::test_tuple_casting().unwrap();
        CodeGenerationTests::test_parameter_handling().unwrap();
    }
}
