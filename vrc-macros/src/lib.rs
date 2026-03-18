use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse_macro_input, punctuated::Punctuated, Data, DeriveInput, Expr, ExprLit, Fields, Lit,
    Meta, Token,
};

/// Derive macro that generates a `validate` method returning
/// `Result<(), std::collections::HashMap<String, String>>`.
///
/// # Supported field attributes
///
/// - `#[validate(max_length = N)]` — byte-length upper bound
/// - `#[validate(min_length = N)]` — byte-length lower bound (implies non-empty when N >= 1)
/// - `#[validate(regex = "PATTERN")]` — match against a `regex_lite::Regex`
/// - `#[validate(https_url)]` — must start with `https://`
/// - `#[validate(max_length = N, https_url)]` — combined
/// - `#[validate(xss_check)]` — reject common XSS vectors in raw text
///
/// For `Option<String>` fields, validation is only applied when the value is `Some`.
/// For `String` fields, validation is always applied.
///
/// The generated method signature:
/// ```ignore
/// pub fn validate(&self) -> Result<(), std::collections::HashMap<String, String>>
/// ```
#[proc_macro_derive(Validate, attributes(validate))]
pub fn derive_validate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_validate(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn impl_validate(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "Validate can only be derived for structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "Validate can only be derived for structs",
            ));
        }
    };

    let mut checks = Vec::new();

    for field in fields {
        let field_ident = field.ident.as_ref().unwrap();
        let field_name = field_ident.to_string();

        // Detect if the field is Option<String>
        let is_option = is_option_type(&field.ty);

        // Collect all #[validate(...)] attributes on this field
        for attr in &field.attrs {
            if !attr.path().is_ident("validate") {
                continue;
            }

            let nested =
                attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

            let mut min_length: Option<usize> = None;
            let mut max_length: Option<usize> = None;
            let mut regex_pattern: Option<String> = None;
            let mut https_url = false;
            let mut xss_check = false;

            for meta in &nested {
                match meta {
                    Meta::NameValue(nv) if nv.path.is_ident("max_length") => {
                        max_length = Some(parse_usize_expr(&nv.value)?);
                    }
                    Meta::NameValue(nv) if nv.path.is_ident("min_length") => {
                        min_length = Some(parse_usize_expr(&nv.value)?);
                    }
                    Meta::NameValue(nv) if nv.path.is_ident("regex") => {
                        regex_pattern = Some(parse_string_expr(&nv.value)?);
                    }
                    Meta::Path(p) if p.is_ident("https_url") => {
                        https_url = true;
                    }
                    Meta::Path(p) if p.is_ident("xss_check") => {
                        xss_check = true;
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            other,
                            "unknown validate attribute; expected one of: \
                             max_length, min_length, regex, https_url, xss_check",
                        ));
                    }
                }
            }

            // Generate the validation code for this field
            let mut field_checks = Vec::new();

            // Length range check (min and/or max)
            if min_length.is_some() || max_length.is_some() {
                let min = min_length.unwrap_or(0);
                let field_name_str = &field_name;
                if let Some(max) = max_length {
                    // Range check: min..=max
                    let msg = if min > 0 {
                        format!("{min}\u{301C}{max}\u{6587}\u{5b57}\u{3067}\u{5165}\u{529b}\u{3057}\u{3066}\u{304f}\u{3060}\u{3055}\u{3044}")
                    } else {
                        format!("{max}\u{6587}\u{5b57}\u{4ee5}\u{5185}\u{3067}\u{5165}\u{529b}\u{3057}\u{3066}\u{304f}\u{3060}\u{3055}\u{3044}")
                    };
                    field_checks.push(quote! {
                        if __val.len() < #min || __val.len() > #max {
                            __errors.insert(
                                #field_name_str.to_owned(),
                                #msg.to_owned(),
                            );
                        }
                    });
                } else {
                    // min_length only
                    let msg = format!("{min}\u{6587}\u{5b57}\u{4ee5}\u{4e0a}\u{3067}\u{5165}\u{529b}\u{3057}\u{3066}\u{304f}\u{3060}\u{3055}\u{3044}");
                    field_checks.push(quote! {
                        if __val.len() < #min {
                            __errors.insert(
                                #field_name_str.to_owned(),
                                #msg.to_owned(),
                            );
                        }
                    });
                }
            }

            // Regex check
            if let Some(ref pattern) = regex_pattern {
                let field_name_str = &field_name;
                let msg = format!("{field_name}\u{306e}\u{5f62}\u{5f0f}\u{304c}\u{6b63}\u{3057}\u{304f}\u{3042}\u{308a}\u{307e}\u{305b}\u{3093}");
                field_checks.push(quote! {
                    {
                        // Build regex at validation time; callers should cache if hot-path
                        static __RE: std::sync::LazyLock<regex_lite::Regex> =
                            std::sync::LazyLock::new(|| {
                                regex_lite::Regex::new(#pattern)
                                    .expect("invalid regex in Validate attribute")
                            });
                        if !__RE.is_match(__val) {
                            __errors.insert(
                                #field_name_str.to_owned(),
                                #msg.to_owned(),
                            );
                        }
                    }
                });
            }

            // HTTPS URL check
            if https_url {
                let field_name_str = &field_name;
                field_checks.push(quote! {
                    if !__val.starts_with("https://") {
                        __errors.insert(
                            #field_name_str.to_owned(),
                            "\u{6709}\u{52b9}\u{306a}HTTPS URL\u{3092}\u{5165}\u{529b}\u{3057}\u{3066}\u{304f}\u{3060}\u{3055}\u{3044}".to_owned(),
                        );
                    }
                });
            }

            // XSS vector check
            if xss_check {
                let field_name_str = &field_name;
                field_checks.push(quote! {
                    {
                        let __lower = __val.to_lowercase();
                        if __lower.contains("javascript:")
                            || __lower.contains("vbscript:")
                            || __lower.contains("onerror")
                            || __lower.contains("onload")
                        {
                            __errors.insert(
                                #field_name_str.to_owned(),
                                "\u{5371}\u{967a}\u{306a}\u{30b3}\u{30f3}\u{30c6}\u{30f3}\u{30c4}\u{304c}\u{691c}\u{51fa}\u{3055}\u{308c}\u{307e}\u{3057}\u{305f}".to_owned(),
                            );
                        }
                    }
                });
            }

            // Wrap the checks depending on whether the field is Option<String> or String
            if !field_checks.is_empty() {
                if is_option {
                    checks.push(quote! {
                        if let Some(ref __val) = self.#field_ident {
                            #(#field_checks)*
                        }
                    });
                } else {
                    checks.push(quote! {
                        {
                            let __val: &str = &self.#field_ident;
                            #(#field_checks)*
                        }
                    });
                }
            }
        }
    }

    Ok(quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            /// Validate fields according to `#[validate(...)]` attributes.
            /// Returns `Ok(())` when all validations pass, or `Err(map)` with
            /// field-name to error-message pairs.
            pub fn validate(&self) -> ::std::result::Result<(), ::std::collections::HashMap<String, String>> {
                let mut __errors: ::std::collections::HashMap<String, String> =
                    ::std::collections::HashMap::new();
                #(#checks)*
                if __errors.is_empty() {
                    Ok(())
                } else {
                    Err(__errors)
                }
            }
        }
    })
}

/// Derive macro that generates `fn error_code(&self) -> &'static str` from
/// `#[code("ERR-XXX-NNN")]` attributes on each variant.
///
/// Also checks at compile time that no two variants share the same code.
///
/// ```ignore
/// #[derive(ErrorCode)]
/// pub enum ProfileError {
///     #[code("ERR-PROF-001")] Validation(HashMap<String, String>),
///     #[code("ERR-PROF-002")] BioDangerous,
/// }
/// ```
#[proc_macro_derive(ErrorCode, attributes(code))]
pub fn derive_error_code(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_error_code(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn impl_error_code(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let variants = match &input.data {
        Data::Enum(data) => &data.variants,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "ErrorCode can only be derived for enums",
            ));
        }
    };

    let mut arms = Vec::new();
    let mut seen_codes: Vec<(String, proc_macro2::Span)> = Vec::new();

    for variant in variants {
        let var_ident = &variant.ident;

        // Find the #[code("...")] attribute
        let code_attr = variant.attrs.iter().find(|a| a.path().is_ident("code"));

        let code_str = match code_attr {
            Some(attr) => {
                let lit: Lit = attr.parse_args()?;
                match lit {
                    Lit::Str(s) => s.value(),
                    _ => {
                        return Err(syn::Error::new_spanned(
                            attr,
                            "expected a string literal in #[code(\"...\")]",
                        ));
                    }
                }
            }
            None => {
                return Err(syn::Error::new_spanned(
                    variant,
                    format!(
                        "variant `{}` is missing a #[code(\"...\")] attribute",
                        var_ident
                    ),
                ));
            }
        };

        // Check for duplicate codes at compile time
        if let Some((_, prev_span)) = seen_codes.iter().find(|(c, _)| c == &code_str) {
            let mut err = syn::Error::new_spanned(
                code_attr.unwrap(),
                format!("duplicate error code: \"{code_str}\""),
            );
            err.combine(syn::Error::new(
                *prev_span,
                format!("error code \"{code_str}\" first used here"),
            ));
            return Err(err);
        }
        seen_codes.push((code_str.clone(), variant.ident.span()));

        // Generate the match arm — handle all field patterns
        let pattern = match &variant.fields {
            Fields::Unit => quote! { Self::#var_ident },
            Fields::Unnamed(_) => quote! { Self::#var_ident(..) },
            Fields::Named(_) => quote! { Self::#var_ident { .. } },
        };

        arms.push(quote! {
            #pattern => #code_str,
        });
    }

    Ok(quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            /// Return the unique error code string for this variant.
            pub fn error_code(&self) -> &'static str {
                match self {
                    #(#arms)*
                }
            }
        }
    })
}

/// Attribute macro that currently passes handler functions through unchanged.
///
/// This is reserved for future expansion per ADR-004 to generate Axum handler
/// boilerplate, OpenAPI metadata, and role-based extraction. Currently a no-op
/// so existing `#[handler]` annotations compile without error.
#[proc_macro_attribute]
pub fn handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Pass-through — the handler attribute is a future extension point.
    // Implementing full route-generation requires tight coupling to the Axum
    // version and the project's auth extractors, which are still evolving.
    item
}

// ── helpers ──────────────────────────────────────────────────────────

/// Return `true` if `ty` is `Option<_>`.
fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last() {
            return seg.ident == "Option";
        }
    }
    false
}

fn parse_usize_expr(expr: &Expr) -> syn::Result<usize> {
    if let Expr::Lit(ExprLit {
        lit: Lit::Int(lit), ..
    }) = expr
    {
        lit.base10_parse::<usize>()
    } else {
        Err(syn::Error::new_spanned(expr, "expected an integer literal"))
    }
}

fn parse_string_expr(expr: &Expr) -> syn::Result<String> {
    if let Expr::Lit(ExprLit {
        lit: Lit::Str(lit), ..
    }) = expr
    {
        Ok(lit.value())
    } else {
        Err(syn::Error::new_spanned(expr, "expected a string literal"))
    }
}
