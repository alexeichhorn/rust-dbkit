use proc_macro::TokenStream;

#[proc_macro_derive(Model, attributes(model, key, autoincrement, unique, index, has_many, belongs_to, many_to_many))]
pub fn derive_model(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}
