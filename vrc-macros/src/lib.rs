use proc_macro::TokenStream;

/// Placeholder for future procedural macros.
/// Will include #[handler], #[require_role], and other derive macros.
#[proc_macro_attribute]
pub fn handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
