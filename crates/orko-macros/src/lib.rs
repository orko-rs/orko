//! # orko-macros
//!
//! Proc-macros for the [orko](https://github.com/orko-rs/orko) toolkit.
//! One item: [`macro@tool`], which turns a (async) function into a zero-argument
//! constructor returning `Arc<dyn Tool>`.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, FnArg, ItemFn, Pat, Type};

/// Turns a (async) function into a tool constructor: `fn name() -> Arc<dyn Tool>`.
///
/// The function's name becomes the tool name. The description comes from
/// `#[tool(desc = "...")]` when given, otherwise from the function's doc
/// comment. A JSON schema is derived from the parameter types at compile
/// time: `String` → `"string"`, integers → `"integer"`, `f32`/`f64` →
/// `"number"`, `bool` → `"boolean"`, `Vec<T>` → `"array"`, `Option<T>` marks
/// the parameter optional (dropped from `required`), and any other type gets
/// the open schema `{}` — serde still validates it when the tool runs, so any
/// `Deserialize` type works as a parameter. The return type must implement
/// `Display`; `async fn` is supported.
///
/// Expansions name the core crate as `::orko_core`, so it must be a direct
/// dependency of the calling crate.
///
/// ```
/// # use orko_macros::tool;
/// #[tool(desc = "Get the weather for a city.")]
/// fn get_weather(city: String) -> String {
///     format!("sunny in {city}")
/// }
///
/// let t = get_weather(); // Arc<dyn Tool>
/// assert_eq!(t.name(), "get_weather");
/// assert_eq!(t.description(), "Get the weather for a city.");
/// ```
///
/// `async fn` works the same; the await happens inside the future returned by
/// `Tool::call`. In async code, `.await` `call`; anywhere else,
/// `call_blocking` runs either kind of tool to completion:
///
/// ```
/// # use orko_macros::tool;
/// #[tool(desc = "Echo text back, asynchronously.")]
/// async fn echo(text: String) -> String {
///     text
/// }
///
/// let t = echo(); // Arc<dyn Tool>
/// let out = t.call_blocking(serde_json::json!({"text": "hi"}));
/// assert_eq!(out.unwrap(), "hi");
/// ```
#[proc_macro_attribute]
pub fn tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    let desc = match parse_desc(attr) {
        Ok(desc) => desc,
        Err(e) => return e.to_compile_error().into(),
    };
    let func = parse_macro_input!(item as ItemFn);
    expand(func, desc)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Parses the optional `desc = "..."` attribute argument for `#[tool]`.
fn parse_desc(attr: TokenStream) -> syn::Result<Option<String>> {
    if attr.is_empty() {
        return Ok(None);
    }
    let nv: syn::MetaNameValue = syn::parse(attr)?;
    if !nv.path.is_ident("desc") {
        return Err(syn::Error::new_spanned(
            &nv.path,
            "#[tool] supports only `desc = \"...\"`",
        ));
    }
    match &nv.value {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) => Ok(Some(s.value())),
        other => Err(syn::Error::new_spanned(
            other,
            "`desc` must be a string literal",
        )),
    }
}

/// Expands a function into an Orko tool definition.
fn expand(func: ItemFn, desc: Option<String>) -> syn::Result<proc_macro2::TokenStream> {
    let sig = &func.sig;
    if !sig.generics.params.is_empty() || sig.generics.where_clause.is_some() {
        return Err(syn::Error::new_spanned(
            &sig.generics,
            "#[tool] functions cannot be generic",
        ));
    }

    let mut names = Vec::new();
    let mut name_strs = Vec::new();
    let mut types = Vec::new();
    for arg in &sig.inputs {
        let FnArg::Typed(pt) = arg else {
            return Err(syn::Error::new_spanned(
                arg,
                "#[tool] only supports free functions (no self)",
            ));
        };
        let Pat::Ident(pi) = &*pt.pat else {
            return Err(syn::Error::new_spanned(
                &pt.pat,
                "#[tool] parameters must be plain identifiers",
            ));
        };
        names.push(pi.ident.clone());
        name_strs.push(pi.ident.to_string());
        types.push((*pt.ty).clone());
    }

    // Rendered to a JSON string literal here; parsed once per `parameters_schema`
    // call at runtime. Param names are Rust idents, so no JSON escaping needed.
    let schema_json = {
        let mut props = Vec::new();
        let mut required = Vec::new();
        for (name, ty) in name_strs.iter().zip(&types) {
            let (schema, req) = schema_for(ty);
            props.push(format!("\"{name}\":{schema}"));
            if req {
                required.push(format!("\"{name}\""));
            }
        }
        let mut json = String::from("{\"type\":\"object\"");
        if !props.is_empty() {
            json.push_str(&format!(",\"properties\":{{{}}}", props.join(",")));
        }
        if !required.is_empty() {
            json.push_str(&format!(",\"required\":[{}]", required.join(",")));
        }
        json.push('}');
        json
    };

    let description = desc.unwrap_or_else(|| doc_string(&func.attrs));
    let tool_name = sig.ident.to_string();
    let vis = &func.vis;
    let ident = &sig.ident;
    let attrs = &func.attrs;

    // The original function, renamed and tucked inside the constructor.
    let mut inner = func.clone();
    inner.sig.ident = format_ident!("__orko_impl");
    inner.vis = syn::Visibility::Inherited;

    let inner_ident = inner.sig.ident.clone();
    let run = if sig.asyncness.is_some() {
        quote!(#inner_ident(#(#names),*).await)
    } else {
        quote!(#inner_ident(#(#names),*))
    };

    let body = if names.is_empty() {
        // Zero-arg tool: ignore whatever arguments the model sent.
        quote! {
            let _ = args;
            ::std::boxed::Box::pin(async move {
                let __out = #run;
                ::orko_core::Result::Ok(::std::string::ToString::to_string(&__out))
            })
        }
    } else {
        quote! {
            ::std::boxed::Box::pin(async move {
                let mut __args = match args {
                    ::orko_core::__private::serde_json::Value::Object(map) => map,
                    other => {
                        return ::orko_core::Result::Err(::orko_core::Error::ToolExecution(
                            ::std::format!(
                                "tool `{}` expects a JSON object of arguments, got: {}",
                                #tool_name, other,
                            ),
                        ))
                    }
                };
                #(
                    let #names: #types = ::orko_core::__private::serde_json::from_value(
                        __args.remove(#name_strs)
                            .unwrap_or(::orko_core::__private::serde_json::Value::Null),
                    )
                    .map_err(|e| ::orko_core::Error::ToolExecution(::std::format!(
                        "invalid argument `{}` for tool `{}`: {}",
                        #name_strs, #tool_name, e,
                    )))?;
                )*
                let __out = #run;
                ::orko_core::Result::Ok(::std::string::ToString::to_string(&__out))
            })
        }
    };

    Ok(quote! {
        #(#attrs)*
        #vis fn #ident() -> ::std::sync::Arc<dyn ::orko_core::Tool> {
            #inner

            struct __OrkoTool;

            impl ::orko_core::Tool for __OrkoTool {
                fn name(&self) -> &str {
                    #tool_name
                }

                fn description(&self) -> &str {
                    #description
                }

                fn parameters_schema(&self) -> ::orko_core::__private::serde_json::Value {
                    ::orko_core::__private::serde_json::from_str(#schema_json)
                        .expect("#[tool]-generated schema is valid JSON")
                }

                fn call<'a>(
                    &'a self,
                    args: ::orko_core::__private::serde_json::Value,
                ) -> ::std::pin::Pin<::std::boxed::Box<
                    dyn ::std::future::Future<Output = ::orko_core::Result<::std::string::String>>
                        + ::std::marker::Send
                        + 'a,
                >> {
                    #body
                }
            }

            ::std::sync::Arc::new(__OrkoTool)
        }
    })
}

/// Maps a Rust parameter type to JSON schema fragment, is-required.
fn schema_for(ty: &Type) -> (String, bool) {
    if let Some(inner) = generic_arg(ty, "Option") {
        return (schema_for(inner).0, false);
    }
    if let Some(inner) = generic_arg(ty, "Vec") {
        let items = schema_for(inner).0;
        return (format!("{{\"type\":\"array\",\"items\":{items}}}"), true);
    }
    let json_type = match type_ident(ty).as_deref() {
        Some("String" | "str") => "string",
        Some("bool") => "boolean",
        Some(
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128"
            | "usize",
        ) => "integer",
        Some("f32" | "f64") => "number",
        _ => return ("{}".to_owned(), true),
    };
    (format!("{{\"type\":\"{json_type}\"}}"), true)
}

/// Last path-segment ident of a type, if it is a path type (`Option<T>` → `Option`).
fn type_ident(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
        _ => None,
    }
}

/// The `T` in `wrapper<T>`, matching on the last path segment.
fn generic_arg<'a>(ty: &'a Type, wrapper: &str) -> Option<&'a Type> {
    let Type::Path(p) = ty else { return None };
    let seg = p.path.segments.last()?;
    if seg.ident != wrapper {
        return None;
    }
    let syn::PathArguments::AngleBracketed(ab) = &seg.arguments else {
        return None;
    };
    if ab.args.len() != 1 {
        return None;
    }
    match ab.args.first()? {
        syn::GenericArgument::Type(t) => Some(t),
        _ => None,
    }
}

/// Joined, trimmed `#[doc]` lines — the tool description shown to the model.
fn doc_string(attrs: &[syn::Attribute]) -> String {
    let mut lines = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        if let syn::Meta::NameValue(nv) = &attr.meta {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                lines.push(s.value().trim().to_owned());
            }
        }
    }
    lines.join("\n")
}
