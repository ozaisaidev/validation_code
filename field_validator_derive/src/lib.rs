// field_validator_derive/src/lib.rs

extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Type};
use syn::LitStr;

/// Derive macro for ValidateFields trait
///
/// This macro will automatically implement the ValidateFields trait
/// for your struct, identifying required fields based on their type:
/// - Fields with non-optional types (not Option<T>) are considered required
/// - Fields with attributes like #[serde(skip_serializing_if="Option::is_none")]
///   or #[serde(default)] are considered optional
#[proc_macro_derive(ValidateFields, attributes(field_validator))]
pub fn derive_validate_fields(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Get the name of the struct
    let name = &input.ident;

    // Extract the list of field names that should be required
    let required_list = extract_required_fields(&input.data);

    // Convert each field name into a string literal for code generation
    let lits: Vec<LitStr> = required_list
        .into_iter()
        .map(|f| LitStr::new(&f, Span::call_site()))
        .collect();

    // Generate implementation. required_fields returns a static slice of &str.
    let expanded = quote! {
        impl field_validator::ValidateFields for #name {
            fn required_fields() -> &'static [&'static str] {
                &[#(#lits),*]
            }
        }
    };

    // Return the generated code
    TokenStream::from(expanded)
}

/// Extract required fields based on their type and attributes
fn extract_required_fields(data: &Data) -> Vec<String> {
    match data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .filter_map(|field| {
                    let field_name = field.ident.as_ref()?;

                    // Skip if field has #[field_validator(optional)] attribute
                    let has_optional_attr = field.attrs.iter().any(|attr| {
                        attr.path().is_ident("field_validator")
                            && attr
                                .meta
                                .require_list()
                                .ok()
                                .map(|list| list.tokens.to_string().contains("optional"))
                                .unwrap_or(false)
                    });

                    // Skip if field has serde(default) or skip_serializing_if attributes
                    let has_serde_optional = field.attrs.iter().any(|attr| {
                        attr.path().is_ident("serde")
                            && attr
                                .meta
                                .require_list()
                                .ok()
                                .map(|list| {
                                    let tokens = list.tokens.to_string();
                                    tokens.contains("default") || tokens.contains("skip_serializing_if")
                                })
                                .unwrap_or(false)
                    });

                    // Skip if field type is Option<T>
                    let is_option_type = is_option_type(&field.ty);

                    // Include as required if none of the optional criteria are met
                    if !has_optional_attr && !has_serde_optional && !is_option_type {
                        Some(field_name.to_string())
                    } else {
                        None
                    }
                })
                .collect(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// Check if a type is Option<T>
fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if type_path.path.segments.len() == 1 {
            return type_path.path.segments[0].ident == "Option";
        }
    }
    false
}
