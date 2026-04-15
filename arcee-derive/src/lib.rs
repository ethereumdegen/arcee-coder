//! Procedural macros for arcee-code.
//!
//! Provides `#[derive(ToolInput)]` which generates a cached JSON Schema
//! accessor for a tool input struct, eliminating the need to hand-write
//! `input_schema()` functions. The generated `schema()` function returns
//! `&'static serde_json::Value`, cached behind a `OnceLock`.
//!
//! # Supported attributes
//!
//! - `#[tool_input(required)]` — mark a field as required in the schema.
//! - `#[tool_input(desc = "...")]` — provide a description.
//! - `#[tool_input(rename = "new_name")]` — use a different JSON property name.
//! - `#[tool_input(enum_values("a", "b", "c"))]` — enumerate allowed string values.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Lit, Meta, Type};

/// Derive a cached JSON schema accessor for a tool input struct.
#[proc_macro_derive(ToolInput, attributes(tool_input))]
pub fn derive_tool_input(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(named) => &named.named,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "ToolInput can only be derived for structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(
                &input,
                "ToolInput can only be derived for structs",
            )
            .to_compile_error()
            .into();
        }
    };

    let mut property_entries = Vec::new();
    let mut required_fields = Vec::new();

    for field in fields {
        let field_name = match &field.ident {
            Some(i) => i,
            None => continue,
        };

        let mut json_name = field_name.to_string();
        let mut description: Option<String> = None;
        let mut enum_values: Vec<String> = Vec::new();
        let mut explicit_required = false;

        for attr in &field.attrs {
            if !attr.path().is_ident("tool_input") {
                continue;
            }
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("required") {
                    explicit_required = true;
                    Ok(())
                } else if meta.path.is_ident("desc") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(s) = lit {
                        description = Some(s.value());
                    }
                    Ok(())
                } else if meta.path.is_ident("rename") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(s) = lit {
                        json_name = s.value();
                    }
                    Ok(())
                } else if meta.path.is_ident("enum_values") {
                    meta.parse_nested_meta(|inner| {
                        if let Some(ident) = inner.path.get_ident() {
                            enum_values.push(ident.to_string());
                        }
                        Ok(())
                    })
                } else {
                    Ok(())
                }
            });
            // Also support #[tool_input(desc = "...", required)] style — already handled above.
            // Support raw attribute parsing for list-style enum_values
            if let Meta::List(list) = &attr.meta {
                let tokens_str = list.tokens.to_string();
                if tokens_str.contains("enum_values") {
                    // naive extractor: enum_values("a", "b")
                    if let Some(start) = tokens_str.find("enum_values") {
                        if let Some(open) = tokens_str[start..].find('(') {
                            let after = &tokens_str[start + open + 1..];
                            if let Some(close) = after.find(')') {
                                let inner = &after[..close];
                                for part in inner.split(',') {
                                    let trimmed = part.trim().trim_matches('"');
                                    if !trimmed.is_empty() && !enum_values.contains(&trimmed.to_string()) {
                                        enum_values.push(trimmed.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let (json_type, is_option) = map_type(&field.ty);

        if !is_option && explicit_required {
            required_fields.push(json_name.clone());
        } else if !is_option && !explicit_required {
            // By convention, non-Option fields are required unless opted out.
            required_fields.push(json_name.clone());
        }

        let desc = description.unwrap_or_default();
        let enum_tokens = if enum_values.is_empty() {
            quote! {}
        } else {
            let values = enum_values.iter();
            quote! { "enum": [ #( #values ),* ], }
        };

        property_entries.push(quote! {
            properties.insert(
                #json_name.to_string(),
                ::serde_json::json!({
                    "type": #json_type,
                    "description": #desc,
                    #enum_tokens
                }),
            );
        });
    }

    let required_lits = required_fields.iter();

    let expanded = quote! {
        impl #name {
            /// Return the cached JSON schema for this tool input type.
            pub fn schema() -> &'static ::serde_json::Value {
                static CELL: ::std::sync::OnceLock<::serde_json::Value> = ::std::sync::OnceLock::new();
                CELL.get_or_init(|| {
                    let mut properties = ::serde_json::Map::new();
                    #( #property_entries )*
                    ::serde_json::json!({
                        "type": "object",
                        "properties": ::serde_json::Value::Object(properties),
                        "required": [ #( #required_lits ),* ],
                    })
                })
            }
        }
    };

    expanded.into()
}

/// Map a Rust type to a JSON Schema type string.
///
/// Returns `(json_type, is_option)` where `is_option` indicates the field is
/// wrapped in `Option<T>` and therefore optional.
fn map_type(ty: &Type) -> (&'static str, bool) {
    if let Type::Path(tp) = ty {
        let segs: Vec<_> = tp.path.segments.iter().collect();
        if let Some(last) = segs.last() {
            let ident = last.ident.to_string();
            if ident == "Option" {
                // Recurse into the inner type
                if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(inner) = arg {
                            let (inner_ty, _) = map_type(inner);
                            return (inner_ty, true);
                        }
                    }
                }
                return ("string", true);
            }
            return match ident.as_str() {
                "String" | "str" => ("string", false),
                "bool" => ("boolean", false),
                "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64"
                | "usize" => ("integer", false),
                "f32" | "f64" => ("number", false),
                "Vec" => ("array", false),
                _ => ("string", false),
            };
        }
    }
    ("string", false)
}
