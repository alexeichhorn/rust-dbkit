use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Attribute, Field, Fields, Ident, ItemStruct, Meta, Type,
};

#[proc_macro_derive(Model, attributes(model, key, autoincrement, unique, index, has_many, belongs_to, many_to_many))]
pub fn derive_model(_input: TokenStream) -> TokenStream {
    TokenStream::from(quote! {
        compile_error!("dbkit: use #[model] instead of #[derive(Model)]");
    })
}

#[proc_macro_attribute]
pub fn model(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let args = parse_macro_input!(attr with syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated);
    let args = parse_model_args(args);
    match expand_model(args, input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelationKind {
    HasMany,
    BelongsTo,
    ManyToMany,
}

struct RelationInfo {
    field: Field,
    param_ident: Ident,
    state_mod_ident: Ident,
    child_type: Type,
    kind: RelationKind,
}

#[derive(Default)]
struct ModelArgs {
    table: Option<String>,
    schema: Option<String>,
}

fn expand_model(args: ModelArgs, input: ItemStruct) -> syn::Result<TokenStream> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            input.generics,
            "dbkit: #[model] does not support generics yet",
        ));
    }

    let struct_ident = input.ident;
    let model_ident = format_ident!("{}Model", struct_ident);
    let insert_ident = format_ident!("{}Insert", struct_ident);
    let vis = input.vis;

    let table_name = args
        .table
        .unwrap_or_else(|| to_snake_case(&struct_ident.to_string()));
    let schema_name = args.schema;

    let mut primary_key: Option<(Ident, Type)> = None;
    let mut relation_fields = Vec::new();
    let mut output_fields = Vec::new();
    let mut insert_fields = Vec::new();

    let struct_attrs = filter_struct_attrs(&input.attrs);

    let fields = match input.fields {
        Fields::Named(named) => named.named,
        _ => {
            return Err(syn::Error::new_spanned(
                struct_ident,
                "dbkit: #[model] requires a struct with named fields",
            ))
        }
    };

    for field in fields {
        let field_ident = field
            .ident
            .clone()
            .ok_or_else(|| syn::Error::new_spanned(&field, "dbkit: unnamed field"))?;

        let is_relation = has_attr(&field.attrs, "has_many")
            || has_attr(&field.attrs, "belongs_to")
            || has_attr(&field.attrs, "many_to_many");

        let is_key = has_attr(&field.attrs, "key");
        let is_autoincrement = has_attr(&field.attrs, "autoincrement");

        if is_key {
            if primary_key.is_some() {
                return Err(syn::Error::new_spanned(
                    &field_ident,
                    "dbkit: multiple #[key] fields are not supported",
                ));
            }
            primary_key = Some((field_ident.clone(), field.ty.clone()));
        }

        if is_relation {
            let (kind, child_type) = relation_type(&field)?;
            let state_mod_ident = format_ident!(
                "{}_{}_state",
                to_snake_case(&struct_ident.to_string()),
                field_ident
            );
            let param_ident = format_ident!("{}", to_camel_case(&field_ident.to_string()));
            relation_fields.push(RelationInfo {
                field: field.clone(),
                param_ident: param_ident.clone(),
                state_mod_ident,
                child_type,
                kind,
            });

            let cleaned_field = Field {
                attrs: filter_field_attrs(&field.attrs),
                ty: syn::parse_quote!(#param_ident),
                ..field
            };
            output_fields.push(cleaned_field);
            continue;
        }

        let cleaned_field = Field {
            attrs: filter_field_attrs(&field.attrs),
            ..field.clone()
        };
        output_fields.push(cleaned_field.clone());

        if !(is_key && is_autoincrement) {
            insert_fields.push(cleaned_field);
        }
    }

    let table_expr = if let Some(schema) = schema_name {
        quote!(::dbkit::Table::new(#table_name).with_schema(#schema))
    } else {
        quote!(::dbkit::Table::new(#table_name))
    };

    let generics_with_defaults = relation_fields
        .iter()
        .map(|rel| {
            let ident = &rel.param_ident;
            let state_mod = &rel.state_mod_ident;
            quote!(#ident: #state_mod::State = ::dbkit::NotLoaded)
        })
        .collect::<Vec<_>>();

    let impl_generics_params = relation_fields
        .iter()
        .map(|rel| {
            let ident = &rel.param_ident;
            let state_mod = &rel.state_mod_ident;
            quote!(#ident: #state_mod::State)
        })
        .collect::<Vec<_>>();

    let generic_idents = relation_fields
        .iter()
        .map(|rel| &rel.param_ident)
        .collect::<Vec<_>>();

    let struct_generics = if generics_with_defaults.is_empty() {
        quote!()
    } else {
        quote!(<#(#generics_with_defaults),*>)
    };

    let impl_generics = if impl_generics_params.is_empty() {
        quote!()
    } else {
        quote!(<#(#impl_generics_params),*>)
    };

    let struct_type_args = if generic_idents.is_empty() {
        quote!()
    } else {
        quote!(<#(#generic_idents),*>)
    };

    let columns = output_fields
        .iter()
        .filter(|field| !is_relation_field(field, &relation_fields))
        .map(|field| {
            let ident = field.ident.as_ref().expect("field ident");
            let name = ident.to_string();
            let ty = &field.ty;
            quote!(pub const #ident: ::dbkit::Column<#struct_ident, #ty> = ::dbkit::Column::new(Self::TABLE, #name);)
        })
        .collect::<Vec<_>>();

    let primary_key_const = primary_key.as_ref().map(|(ident, ty)| {
        let name = ident.to_string();
        quote!(pub const PRIMARY_KEY: ::dbkit::Column<#struct_ident, #ty> = ::dbkit::Column::new(Self::TABLE, #name);)
    });

    let by_id_fn = primary_key.as_ref().map(|(ident, ty)| {
        quote!(
            pub fn by_id(id: #ty) -> ::dbkit::Select<#struct_ident> {
                Self::query().filter(Self::#ident.eq(id)).limit(1)
            }
        )
    });

    let relation_state_modules = relation_fields.iter().map(|rel| {
        let state_mod = &rel.state_mod_ident;
        let child_type = adjust_type_for_module(&rel.child_type);
        let loaded_type = match rel.kind {
            RelationKind::HasMany | RelationKind::ManyToMany => quote!(Vec<#child_type>),
            RelationKind::BelongsTo => quote!(Option<#child_type>),
        };
        quote!(
            pub mod #state_mod {
                mod sealed {
                    pub trait Sealed {}
                    impl Sealed for ::dbkit::NotLoaded {}
                    impl Sealed for #loaded_type {}
                }
                pub trait State: sealed::Sealed {}
                impl State for ::dbkit::NotLoaded {}
                impl State for #loaded_type {}
            }
        )
    });

    let relation_methods = relation_fields.iter().map(|rel| {
        let field_ident = rel.field.ident.as_ref().expect("field ident");
        let child_type = &rel.child_type;
        let loaded_type: Type = match rel.kind {
            RelationKind::HasMany | RelationKind::ManyToMany => syn::parse_quote!(Vec<#child_type>),
            RelationKind::BelongsTo => syn::parse_quote!(Option<#child_type>),
        };

        let mut other_params = Vec::new();
        let mut type_params = Vec::new();
        for other in &relation_fields {
            if other.field.ident == rel.field.ident {
                type_params.push(quote!(#loaded_type));
            } else {
                let ident = &other.param_ident;
                let state_mod = &other.state_mod_ident;
                other_params.push(quote!(#ident: #state_mod::State));
                type_params.push(quote!(#ident));
            }
        }

        let impl_generics = if other_params.is_empty() {
            quote!()
        } else {
            quote!(<#(#other_params),*>)
        };
        let type_args = if type_params.is_empty() {
            quote!()
        } else {
            quote!(<#(#type_params),*>)
        };

        let (return_ty, body) = match rel.kind {
            RelationKind::HasMany | RelationKind::ManyToMany => {
                (quote!(&[#child_type]), quote!(&self.#field_ident))
            }
            RelationKind::BelongsTo => {
                (quote!(Option<&#child_type>), quote!(self.#field_ident.as_ref()))
            }
        };

        quote!(
            impl #impl_generics #model_ident #type_args {
                pub fn #field_ident(&self) -> #return_ty {
                    #body
                }
            }
        )
    });

    let output = quote! {
        #(#struct_attrs)*
        #vis struct #model_ident #struct_generics {
            #(#output_fields,)*
        }

        #vis type #struct_ident = #model_ident;

        #(#relation_state_modules)*

        impl #impl_generics #model_ident #struct_type_args {
            pub const TABLE: ::dbkit::Table = #table_expr;
            #(#columns)*
            #primary_key_const

            pub fn query() -> ::dbkit::Select<#struct_ident> {
                ::dbkit::Select::new(Self::TABLE)
            }

            #by_id_fn

            pub fn insert() -> ::dbkit::Insert<#struct_ident> {
                ::dbkit::Insert::new(Self::TABLE)
            }

            pub fn update() -> ::dbkit::Update<#struct_ident> {
                ::dbkit::Update::new(Self::TABLE)
            }

            pub fn delete() -> ::dbkit::Delete {
                ::dbkit::Delete::new(Self::TABLE)
            }
        }

        #[derive(Debug, Clone)]
        #vis struct #insert_ident {
            #(#insert_fields,)*
        }

        #(#relation_methods)*
    };

    Ok(output.into())
}

fn parse_model_args(args: syn::punctuated::Punctuated<Meta, syn::Token![,]>) -> ModelArgs {
    let mut out = ModelArgs::default();
    for meta in args {
        if let Meta::NameValue(nv) = meta {
            if nv.path.is_ident("table") {
                if let Some(value) = extract_lit_str(&nv.value) {
                    out.table = Some(value);
                }
            } else if nv.path.is_ident("schema") {
                if let Some(value) = extract_lit_str(&nv.value) {
                    out.schema = Some(value);
                }
            }
        }
    }
    out
}

fn extract_lit_str(expr: &syn::Expr) -> Option<String> {
    if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit), .. }) = expr {
        Some(lit.value())
    } else {
        None
    }
}

fn has_attr(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(name))
}

fn filter_struct_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    let mut kept = Vec::new();
    for attr in attrs {
        if is_model_attr(attr) {
            continue;
        }
        if attr.path().is_ident("derive") {
            if let Ok(mut paths) = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            ) {
                paths = paths
                    .into_iter()
                    .filter(|path| {
                        !path
                            .segments
                            .last()
                            .map(|seg| seg.ident == "Model")
                            .unwrap_or(false)
                    })
                    .collect();
                if paths.is_empty() {
                    continue;
                }
                let new_attr = quote!(#[derive(#paths)]);
                kept.push(syn::parse2(new_attr).expect("derive attr"));
                continue;
            }
        }
        kept.push(attr.clone());
    }
    kept
}

fn filter_field_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter(|attr| !is_field_orm_attr(attr))
        .cloned()
        .collect()
}

fn is_field_orm_attr(attr: &Attribute) -> bool {
    let name = attr.path().get_ident().map(|ident| ident.to_string());
    matches!(
        name.as_deref(),
        Some("key")
            | Some("autoincrement")
            | Some("unique")
            | Some("index")
            | Some("has_many")
            | Some("belongs_to")
            | Some("many_to_many")
    )
}

fn is_model_attr(attr: &Attribute) -> bool {
    attr.path().is_ident("model")
}

fn relation_type(field: &Field) -> syn::Result<(RelationKind, Type)> {
    let kind = if has_attr(&field.attrs, "has_many") {
        RelationKind::HasMany
    } else if has_attr(&field.attrs, "belongs_to") {
        RelationKind::BelongsTo
    } else if has_attr(&field.attrs, "many_to_many") {
        RelationKind::ManyToMany
    } else {
        return Err(syn::Error::new_spanned(
            field,
            "dbkit: missing relation attribute",
        ));
    };

    let child_type = match &field.ty {
        Type::Path(path) => {
            let segment = path
                .path
                .segments
                .last()
                .ok_or_else(|| syn::Error::new_spanned(&field.ty, "dbkit: invalid type"))?;
            let expected = match kind {
                RelationKind::HasMany => "HasMany",
                RelationKind::BelongsTo => "BelongsTo",
                RelationKind::ManyToMany => "ManyToMany",
            };
            if segment.ident != expected {
                return Err(syn::Error::new_spanned(
                    &segment.ident,
                    format!("dbkit: expected {} marker type", expected),
                ));
            }
            match &segment.arguments {
                syn::PathArguments::AngleBracketed(args) => {
                    let ty = args.args.iter().find_map(|arg| match arg {
                        syn::GenericArgument::Type(ty) => Some(ty.clone()),
                        _ => None,
                    });
                    ty.ok_or_else(|| syn::Error::new_spanned(&segment, "dbkit: missing type"))?
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        &segment.arguments,
                        "dbkit: expected generic argument",
                    ))
                }
            }
        }
        _ => {
            return Err(syn::Error::new_spanned(
                &field.ty,
                "dbkit: relation marker must be a type path",
            ))
        }
    };

    Ok((kind, child_type))
}

fn is_relation_field(field: &Field, rels: &[RelationInfo]) -> bool {
    rels.iter()
        .any(|rel| rel.field.ident == field.ident)
}

fn to_snake_case(name: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if idx > 0 {
                out.push('_');
            }
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn to_camel_case(name: &str) -> String {
    let mut out = String::new();
    let mut uppercase_next = true;
    for ch in name.chars() {
        if ch == '_' {
            uppercase_next = true;
            continue;
        }
        if uppercase_next {
            for up in ch.to_uppercase() {
                out.push(up);
            }
            uppercase_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn adjust_type_for_module(ty: &Type) -> proc_macro2::TokenStream {
    match ty {
        Type::Path(path) => {
            if path.qself.is_some() {
                return quote!(#ty);
            }
            if path.path.leading_colon.is_some() {
                return quote!(#ty);
            }
            let first = path.path.segments.first().map(|seg| seg.ident.to_string());
            if matches!(first.as_deref(), Some("crate") | Some("self") | Some("super")) {
                quote!(#ty)
            } else if path.path.segments.len() == 1 {
                quote!(super::#ty)
            } else {
                quote!(#ty)
            }
        }
        _ => quote!(#ty),
    }
}
