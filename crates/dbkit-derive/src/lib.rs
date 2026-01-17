use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Attribute, Field, Fields, Ident, ItemStruct, Meta, Type,
};
use syn::parse::Parser;

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
    belongs_to_key: Option<Ident>,
    belongs_to_ref: Option<Ident>,
}

struct ScalarFieldInfo {
    field: Field,
    ident: Ident,
    ty: Type,
    is_autoincrement: bool,
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
    let mut scalar_fields = Vec::new();

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
            let param_ident = format_ident!("{}Rel", to_camel_case(&field_ident.to_string()));
            let (belongs_to_key, belongs_to_ref) = if kind == RelationKind::BelongsTo {
                let (key, references) = parse_belongs_to_args(&field.attrs)?;
                (Some(key), Some(references))
            } else {
                (None, None)
            };

            relation_fields.push(RelationInfo {
                field: field.clone(),
                param_ident: param_ident.clone(),
                state_mod_ident,
                child_type,
                kind,
                belongs_to_key,
                belongs_to_ref,
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
            insert_fields.push(cleaned_field.clone());
        }

        scalar_fields.push(ScalarFieldInfo {
            field: cleaned_field,
            ident: field_ident,
            ty: field.ty.clone(),
            is_autoincrement,
        });
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
            let ty = option_inner_type(&field.ty).unwrap_or_else(|| field.ty.clone());
            quote!(pub const #ident: ::dbkit::Column<#struct_ident, #ty> = ::dbkit::Column::new(Self::TABLE, #name);)
        })
        .collect::<Vec<_>>();

    let insert_values = insert_fields.iter().map(|field| {
        let ident = field.ident.as_ref().expect("field ident");
        quote!(insert = insert.value(Self::#ident, values.#ident);)
    });

    let active_ident = format_ident!("{}Active", struct_ident);

    let active_fields = scalar_fields.iter().map(|field| {
        let ident = &field.ident;
        let vis = &field.field.vis;
        let ty = option_inner_type(&field.ty).unwrap_or_else(|| field.ty.clone());
        quote!(#vis #ident: ::dbkit::ActiveValue<#ty>)
    });

    let active_from_model = scalar_fields.iter().map(|field| {
        let ident = &field.ident;
        quote!(#ident: ::dbkit::ActiveValue::from(#ident))
    });

    let active_destructure = scalar_fields
        .iter()
        .map(|field| field.ident.clone())
        .collect::<Vec<_>>();

    let active_insert_steps = scalar_fields.iter().map(|field| {
        let ident = &field.ident;
        let name = ident.to_string();
        let ty = option_inner_type(&field.ty).unwrap_or_else(|| field.ty.clone());
        let is_option = option_inner_type(&field.ty).is_some();
        let required = !field.is_autoincrement && !is_option;
        let required_check = if required {
            quote!(return Err(::dbkit::Error::Decode(format!("missing required field: {}", #name)));)
        } else {
            quote!()
        };
        quote!(
            match #ident {
                ::dbkit::ActiveValue::Unset => {
                    #required_check
                }
                ::dbkit::ActiveValue::Set(value) => {
                    insert = insert.value(#struct_ident::#ident, value);
                }
                ::dbkit::ActiveValue::Null => {
                    insert = insert.value(#struct_ident::#ident, None::<#ty>);
                }
            }
        )
    });

    let active_insert_fn = quote!(
        pub async fn insert(
            self,
            ex: &mut (impl ::dbkit::Executor + Send),
        ) -> Result<#struct_ident, ::dbkit::Error> {
            let Self { #(#active_destructure,)* } = self;
            let mut insert = ::dbkit::Insert::new(#struct_ident::TABLE);
            #(#active_insert_steps)*
            let insert = insert.returning_all();
            let row = ::dbkit::InsertExt::one(insert, ex).await?;
            row.ok_or(::dbkit::Error::NotFound)
        }
    );

    let active_update_fn = if let Some((pk_ident, _pk_ty)) = &primary_key {
        let pk_name = pk_ident.to_string();
        let update_steps = scalar_fields.iter().filter(|field| field.ident != *pk_ident).map(|field| {
            let ident = &field.ident;
            let ty = option_inner_type(&field.ty).unwrap_or_else(|| field.ty.clone());
            quote!(
                match #ident {
                    ::dbkit::ActiveValue::Unset => {}
                    ::dbkit::ActiveValue::Set(value) => {
                        update = update.set(#struct_ident::#ident, value);
                        any_set = true;
                    }
                    ::dbkit::ActiveValue::Null => {
                        update = update.set(#struct_ident::#ident, None::<#ty>);
                        any_set = true;
                    }
                }
            )
        });
        quote!(
            pub async fn update(
                self,
                ex: &mut (impl ::dbkit::Executor + Send),
            ) -> Result<#struct_ident, ::dbkit::Error> {
                let Self { #(#active_destructure,)* } = self;
                let pk_value = match #pk_ident {
                    ::dbkit::ActiveValue::Set(value) => value,
                    ::dbkit::ActiveValue::Null | ::dbkit::ActiveValue::Unset => {
                        return Err(::dbkit::Error::Decode(format!(
                            "missing required field: {}",
                            #pk_name
                        )));
                    }
                };
                let mut update = ::dbkit::Update::new(#struct_ident::TABLE);
                let mut any_set = false;
                #(#update_steps)*
                if !any_set {
                    return Err(::dbkit::Error::Decode("no fields set for update".to_string()));
                }
                let update = update
                    .filter(#struct_ident::#pk_ident.eq(pk_value))
                    .returning_all();
                let mut rows = ::dbkit::UpdateExt::all(update, ex).await?;
                rows.pop().ok_or(::dbkit::Error::NotFound)
            }
        )
    } else {
        quote!()
    };

    let into_active_fn = quote!(
        pub fn into_active(self) -> #active_ident {
            let Self { #(#active_destructure,)* .. } = self;
            #active_ident {
                #(#active_from_model,)*
            }
        }
    );

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

    let any_state_ident = format_ident!("{}AnyState", struct_ident);

    let relation_state_modules = relation_fields.iter().map(|rel| {
        let state_mod = &rel.state_mod_ident;
        let (sealed_impl, state_impl) = match rel.kind {
            RelationKind::HasMany | RelationKind::ManyToMany => (
                quote!(impl<T> Sealed for Vec<T> {}),
                quote!(impl<T> State for Vec<T> {}),
            ),
            RelationKind::BelongsTo => (
                quote!(impl<T> Sealed for Option<T> {}),
                quote!(impl<T> State for Option<T> {}),
            ),
        };
        quote!(
            pub mod #state_mod {
                mod sealed {
                    pub trait Sealed {}
                    impl Sealed for ::dbkit::NotLoaded {}
                    #sealed_impl
                }
                pub trait State: sealed::Sealed {}
                impl State for ::dbkit::NotLoaded {}
                #state_impl
            }
        )
    });

    let relation_methods = relation_fields.iter().map(|rel| {
        let field_ident = rel.field.ident.as_ref().expect("field ident");
        let method_ident = format_ident!("{}_loaded", field_ident);
        let item_ident = format_ident!("{}Item", to_camel_case(&field_ident.to_string()));
        let loaded_type: Type = match rel.kind {
            RelationKind::HasMany | RelationKind::ManyToMany => syn::parse_quote!(Vec<#item_ident>),
            RelationKind::BelongsTo => syn::parse_quote!(Option<#item_ident>),
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

        let mut impl_params = Vec::new();
        impl_params.push(quote!(#item_ident));
        impl_params.extend(other_params);

        let impl_generics = if impl_params.is_empty() {
            quote!()
        } else {
            quote!(<#(#impl_params),*>)
        };
        let type_args = if type_params.is_empty() {
            quote!()
        } else {
            quote!(<#(#type_params),*>)
        };

        let (return_ty, body) = match rel.kind {
            RelationKind::HasMany | RelationKind::ManyToMany => {
                (quote!(&[#item_ident]), quote!(&self.#field_ident))
            }
            RelationKind::BelongsTo => {
                (quote!(Option<&#item_ident>), quote!(self.#field_ident.as_ref()))
            }
        };

        quote!(
            impl #impl_generics #model_ident #type_args {
                pub fn #method_ident(&self) -> #return_ty {
                    #body
                }
            }
        )
    });

    let model_value_arms = output_fields
        .iter()
        .filter(|field| !is_relation_field(field, &relation_fields))
        .map(|field| {
            let ident = field.ident.as_ref().expect("field ident");
            let name = ident.to_string();
            quote!(#name => Some(self.#ident.clone().into()),)
        });

    let model_value_impl = quote!(
        impl #impl_generics ::dbkit::ModelValue for #model_ident #struct_type_args {
            fn column_value(&self, column: ::dbkit::ColumnRef) -> Option<::dbkit::Value> {
                if column.table.name != Self::TABLE.name {
                    return None;
                }
                match column.name {
                    #(#model_value_arms)*
                    _ => None,
                }
            }
        }
    );

    let from_row_generics = relation_fields.iter().map(|rel| {
        let ident = &rel.param_ident;
        let state_mod = &rel.state_mod_ident;
        quote!(#ident: #state_mod::State + Default)
    });

    let from_row_impl_generics = if relation_fields.is_empty() {
        quote!(<'r>)
    } else {
        quote!(<'r, #(#from_row_generics),*>)
    };

    let from_row_fields = output_fields.iter().map(|field| {
        let ident = field.ident.as_ref().expect("field ident");
        if is_relation_field(field, &relation_fields) {
            quote!(#ident: Default::default())
        } else {
            let name = ident.to_string();
            quote!(#ident: ::dbkit::sqlx::Row::try_get(row, #name)?)
        }
    });

    let from_row_impl = quote!(
        impl #from_row_impl_generics ::dbkit::sqlx::FromRow<'r, ::dbkit::sqlx::postgres::PgRow>
            for #model_ident #struct_type_args
        {
            fn from_row(row: &'r ::dbkit::sqlx::postgres::PgRow) -> Result<Self, ::dbkit::sqlx::Error> {
                Ok(Self {
                    #(#from_row_fields,)*
                })
            }
        }
    );

    let set_relation_impls = relation_fields.iter().map(|rel| {
        let field_ident = rel.field.ident.as_ref().expect("field ident");
        let child_type = &rel.child_type;
        let item_ident = format_ident!("{}Item", to_camel_case(&field_ident.to_string()));
        let (value_ty, rel_ty) = match rel.kind {
            RelationKind::HasMany | RelationKind::ManyToMany => (
                quote!(Vec<#item_ident>),
                quote!(::dbkit::rel::HasMany<#struct_ident, #child_type>),
            ),
            RelationKind::BelongsTo => (
                quote!(Option<#item_ident>),
                quote!(::dbkit::rel::BelongsTo<#struct_ident, #child_type>),
            ),
        };

        let mut other_params = Vec::new();
        let mut type_params = Vec::new();
        for other in &relation_fields {
            if other.field.ident == rel.field.ident {
                type_params.push(value_ty.clone());
            } else {
                let ident = &other.param_ident;
                let state_mod = &other.state_mod_ident;
                other_params.push(quote!(#ident: #state_mod::State));
                type_params.push(quote!(#ident));
            }
        }

        let mut impl_params = Vec::new();
        impl_params.push(quote!(#item_ident));
        impl_params.extend(other_params);

        let impl_generics = if impl_params.is_empty() {
            quote!()
        } else {
            quote!(<#(#impl_params),*>)
        };
        let type_args = if type_params.is_empty() {
            quote!()
        } else {
            quote!(<#(#type_params),*>)
        };

        quote!(
            impl #impl_generics ::dbkit::SetRelation<#rel_ty, #value_ty> for #model_ident #type_args {
                fn set_relation(&mut self, _rel: #rel_ty, value: #value_ty) -> Result<(), ::dbkit::Error> {
                    self.#field_ident = value;
                    Ok(())
                }
            }
        )
    });

    let load_methods = relation_fields.iter().map(|rel| {
        let field_ident = rel.field.ident.as_ref().expect("field ident");
        let child_type = &rel.child_type;
        let rel_type = match rel.kind {
            RelationKind::HasMany | RelationKind::ManyToMany => {
                quote!(::dbkit::rel::HasMany<#struct_ident, #child_type>)
            }
            RelationKind::BelongsTo => quote!(::dbkit::rel::BelongsTo<#struct_ident, #child_type>),
        };
        let loaded_type = match rel.kind {
            RelationKind::HasMany | RelationKind::ManyToMany => quote!(Vec<#child_type>),
            RelationKind::BelongsTo => quote!(Option<#child_type>),
        };
        let loader_fn = match rel.kind {
            RelationKind::HasMany | RelationKind::ManyToMany => {
                quote!(::dbkit::runtime::load_selectin_has_many)
            }
            RelationKind::BelongsTo => quote!(::dbkit::runtime::load_selectin_belongs_to),
        };

        let mut other_params = Vec::new();
        let mut type_params = Vec::new();
        let mut out_params = Vec::new();
        for other in &relation_fields {
            if other.field.ident == rel.field.ident {
                type_params.push(quote!(::dbkit::NotLoaded));
                out_params.push(loaded_type.clone());
            } else {
                let ident = &other.param_ident;
                let state_mod = &other.state_mod_ident;
                other_params.push(quote!(#ident: #state_mod::State));
                type_params.push(quote!(#ident));
                out_params.push(quote!(#ident));
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
        let out_type = if out_params.is_empty() {
            quote!(#model_ident)
        } else {
            quote!(#model_ident<#(#out_params),*>)
        };
        let out_construct = if out_params.is_empty() {
            quote!(#model_ident)
        } else {
            quote!(#model_ident::<#(#out_params),*>)
        };

        let destructure_fields = output_fields.iter().map(|field| {
            let ident = field.ident.as_ref().expect("field ident");
            if ident == field_ident {
                quote!(#ident: _)
            } else {
                quote!(#ident)
            }
        });

        let build_fields = output_fields.iter().map(|field| {
            let ident = field.ident.as_ref().expect("field ident");
            if ident == field_ident {
                quote!(#ident: Default::default())
            } else {
                quote!(#ident)
            }
        });

        quote!(
            impl #impl_generics #model_ident #type_args {
                pub async fn load(
                    self,
                    rel: #rel_type,
                    mut ex: impl ::dbkit::Executor + Send,
                ) -> Result<#out_type, ::dbkit::Error> {
                    let Self { #(#destructure_fields,)* } = self;
                    let mut out = #out_construct {
                        #(#build_fields,)*
                    };
                    let mut rows = vec![out];
                    #loader_fn(&mut ex, &mut rows, rel, &::dbkit::load::NoLoad).await?;
                    Ok(rows.pop().expect("loaded row"))
                }
            }
        )
    });

    let relation_consts = relation_fields.iter().filter_map(|rel| {
        let field_ident = rel.field.ident.as_ref().expect("field ident");
        let child_type = &rel.child_type;
        match rel.kind {
            RelationKind::HasMany => Some(quote!(
                pub const #field_ident: ::dbkit::rel::HasMany<#struct_ident, #child_type> =
                    ::dbkit::rel::HasMany::new(
                        <#child_type as ::dbkit::rel::BelongsToSpec<#struct_ident>>::PARENT_TABLE,
                        <#child_type as ::dbkit::rel::BelongsToSpec<#struct_ident>>::CHILD_TABLE,
                        <#child_type as ::dbkit::rel::BelongsToSpec<#struct_ident>>::PARENT_KEY,
                        <#child_type as ::dbkit::rel::BelongsToSpec<#struct_ident>>::CHILD_KEY,
                    );
            )),
            RelationKind::BelongsTo => {
                let key = rel.belongs_to_key.as_ref().expect("belongs_to key");
                let references = rel.belongs_to_ref.as_ref().expect("belongs_to references");
                Some(quote!(
                    pub const #field_ident: ::dbkit::rel::BelongsTo<#struct_ident, #child_type> =
                        ::dbkit::rel::BelongsTo::new(
                            Self::TABLE,
                            #child_type::TABLE,
                            Self::#key.as_ref(),
                            #child_type::#references.as_ref(),
                        );
                ))
            }
            RelationKind::ManyToMany => None,
        }
    });

    let belongs_to_specs = relation_fields.iter().filter_map(|rel| {
        if rel.kind != RelationKind::BelongsTo {
            return None;
        }
        let parent_type = &rel.child_type;
        let key = rel.belongs_to_key.as_ref().expect("belongs_to key");
        let references = rel.belongs_to_ref.as_ref().expect("belongs_to references");
        Some(quote!(
            impl #impl_generics ::dbkit::rel::BelongsToSpec<#parent_type> for #model_ident #struct_type_args {
                const CHILD_TABLE: ::dbkit::Table = Self::TABLE;
                const PARENT_TABLE: ::dbkit::Table = #parent_type::TABLE;
                const CHILD_KEY: ::dbkit::ColumnRef = Self::#key.as_ref();
                const PARENT_KEY: ::dbkit::ColumnRef = #parent_type::#references.as_ref();
            }
        ))
    });

    let apply_load_impls = relation_fields.iter().flat_map(|rel| {
        let child_type = &rel.child_type;
        let rel_type = match rel.kind {
            RelationKind::HasMany => quote!(::dbkit::rel::HasMany<#struct_ident, #child_type>),
            RelationKind::BelongsTo => quote!(::dbkit::rel::BelongsTo<#struct_ident, #child_type>),
            RelationKind::ManyToMany => return Vec::new().into_iter(),
        };

        let loaded_child = quote!(<Nested as ::dbkit::load::ApplyLoad<#child_type>>::Out2);
        let loaded_param = match rel.kind {
            RelationKind::HasMany | RelationKind::ManyToMany => quote!(Vec<#loaded_child>),
            RelationKind::BelongsTo => quote!(Option<#loaded_child>),
        };

        let mut out_params = Vec::new();
        for other in &relation_fields {
            if other.field.ident == rel.field.ident {
                out_params.push(loaded_param.clone());
            } else {
                let ident = &other.param_ident;
                out_params.push(quote!(#ident));
            }
        }

        let model_type = if generic_idents.is_empty() {
            quote!(#model_ident)
        } else {
            quote!(#model_ident<#(#generic_idents),*>)
        };
        let out_type = if out_params.is_empty() {
            quote!(#model_ident)
        } else {
            quote!(#model_ident<#(#out_params),*>)
        };

        let mut apply_generics = Vec::new();
        apply_generics.push(quote!(Nested));
        apply_generics.extend(impl_generics_params.iter().cloned());
        let apply_generics = if apply_generics.is_empty() {
            quote!()
        } else {
            quote!(<#(#apply_generics),*>)
        };

        let mut items = Vec::new();
        for strategy in ["SelectIn", "Joined"] {
            let load_ty = if strategy == "SelectIn" {
                quote!(::dbkit::load::SelectIn<#rel_type, Nested>)
            } else {
                quote!(::dbkit::load::Joined<#rel_type, Nested>)
            };
            items.push(quote!(
                impl #apply_generics ::dbkit::load::ApplyLoad<#model_type> for #load_ty
                where
                    Nested: ::dbkit::load::ApplyLoad<#child_type>,
                {
                    type Out2 = #out_type;
                }
            ));
        }
        items.into_iter()
    });

    let run_load_impls = relation_fields.iter().flat_map(|rel| {
        let child_type = &rel.child_type;
        let rel_type = match rel.kind {
            RelationKind::HasMany => quote!(::dbkit::rel::HasMany<#struct_ident, #child_type>),
            RelationKind::BelongsTo => quote!(::dbkit::rel::BelongsTo<#struct_ident, #child_type>),
            RelationKind::ManyToMany => return Vec::new().into_iter(),
        };

        let loaded_child = quote!(<Nested as ::dbkit::load::ApplyLoad<#child_type>>::Out2);
        let loaded_param = match rel.kind {
            RelationKind::HasMany | RelationKind::ManyToMany => quote!(Vec<#loaded_child>),
            RelationKind::BelongsTo => quote!(Option<#loaded_child>),
        };

        let mut out_params = Vec::new();
        for other in &relation_fields {
            if other.field.ident == rel.field.ident {
                out_params.push(loaded_param.clone());
            } else {
                let ident = &other.param_ident;
                out_params.push(quote!(#ident));
            }
        }

        let out_type = if out_params.is_empty() {
            quote!(#model_ident)
        } else {
            quote!(#model_ident<#(#out_params),*>)
        };

        let mut apply_generics = Vec::new();
        apply_generics.push(quote!(Nested));
        for other in &relation_fields {
            if other.field.ident == rel.field.ident {
                continue;
            }
            let ident = &other.param_ident;
            let state_mod = &other.state_mod_ident;
            apply_generics.push(quote!(#ident: #state_mod::State));
        }
        let apply_generics = if apply_generics.is_empty() {
            quote!()
        } else {
            quote!(<#(#apply_generics),*>)
        };

        let (child_bounds, loader_fn) = match rel.kind {
            RelationKind::HasMany | RelationKind::ManyToMany => (
                quote!(#loaded_child: ::dbkit::ModelValue + for<'r> ::dbkit::sqlx::FromRow<'r, ::dbkit::sqlx::postgres::PgRow> + Send + Unpin,),
                quote!(::dbkit::runtime::load_selectin_has_many),
            ),
            RelationKind::BelongsTo => (
                quote!(#loaded_child: ::dbkit::ModelValue + Clone + for<'r> ::dbkit::sqlx::FromRow<'r, ::dbkit::sqlx::postgres::PgRow> + Send + Unpin,),
                quote!(::dbkit::runtime::load_selectin_belongs_to),
            ),
        };

        let joined_loader_fn = match rel.kind {
            RelationKind::HasMany | RelationKind::ManyToMany => quote!(::dbkit::runtime::load_joined_has_many),
            RelationKind::BelongsTo => quote!(::dbkit::runtime::load_joined_belongs_to),
        };

        let mut items = Vec::new();
        for (strategy, loader) in [
            ("SelectIn", loader_fn),
            ("Joined", joined_loader_fn),
        ] {
            let load_ty = if strategy == "SelectIn" {
                quote!(::dbkit::load::SelectIn<#rel_type, Nested>)
            } else {
                quote!(::dbkit::load::Joined<#rel_type, Nested>)
            };

            items.push(quote!(
                impl #apply_generics ::dbkit::runtime::RunLoad<#out_type> for #load_ty
                where
                    Nested: ::dbkit::load::ApplyLoad<#child_type> + ::dbkit::runtime::RunLoads<#loaded_child> + Sync,
                    #out_type: ::dbkit::ModelValue + ::dbkit::SetRelation<#rel_type, #loaded_param>,
                    #child_bounds
                {
                    fn run<'e, E>(
                        &'e self,
                        ex: &'e mut E,
                        rows: &'e mut [#out_type],
                    ) -> ::dbkit::executor::BoxFuture<'e, Result<(), ::dbkit::Error>>
                    where
                        E: ::dbkit::Executor + Send + 'e,
                    {
                        #loader(ex, rows, self.rel.clone(), &self.nested)
                    }
                }
            ));
        }
        items.into_iter()
    });

    let output = quote! {
        #(#struct_attrs)*
        #[derive(Debug, Clone)]
        #vis struct #model_ident #struct_generics {
            #(#output_fields,)*
        }

        #vis type #struct_ident = #model_ident;

        #(#relation_state_modules)*

        #vis trait #any_state_ident {}
        impl #impl_generics #any_state_ident for #model_ident #struct_type_args {}

        impl #impl_generics #model_ident #struct_type_args {
            pub const TABLE: ::dbkit::Table = #table_expr;
            #(#columns)*
            #primary_key_const
            #(#relation_consts)*

            pub fn query() -> ::dbkit::Select<#struct_ident> {
                ::dbkit::Select::new(Self::TABLE)
            }

            #by_id_fn

            pub fn insert(values: #insert_ident) -> ::dbkit::Insert<#struct_ident> {
                let mut insert = ::dbkit::Insert::new(Self::TABLE);
                #(#insert_values)*
                insert
            }

            pub fn update() -> ::dbkit::Update<#struct_ident> {
                ::dbkit::Update::new(Self::TABLE)
            }

            pub fn delete() -> ::dbkit::Delete {
                ::dbkit::Delete::new(Self::TABLE)
            }

            pub fn new_active() -> #active_ident {
                #active_ident::new()
            }

            #into_active_fn
        }

        #[derive(Debug, Clone)]
        #vis struct #insert_ident {
            #(#insert_fields,)*
        }

        #[derive(Debug, Clone, Default)]
        #vis struct #active_ident {
            #(#active_fields,)*
        }

        impl #active_ident {
            pub fn new() -> Self {
                Self::default()
            }

            #active_insert_fn
            #active_update_fn
        }

        #(#relation_methods)*
        #model_value_impl
        #from_row_impl
        #(#set_relation_impls)*
        #(#load_methods)*
        #(#belongs_to_specs)*
        #(#apply_load_impls)*
        #(#run_load_impls)*
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

fn parse_belongs_to_args(attrs: &[Attribute]) -> syn::Result<(Ident, Ident)> {
    for attr in attrs {
        if !attr.path().is_ident("belongs_to") {
            continue;
        }
        let args = attr.parse_args_with(
            syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
        )?;
        let mut key = None;
        let mut references = None;
        for meta in args {
            if let Meta::NameValue(nv) = meta {
                if nv.path.is_ident("key") {
                    key = extract_ident(&nv.value);
                } else if nv.path.is_ident("references") {
                    references = extract_ident(&nv.value);
                }
            }
        }
        if let (Some(key), Some(references)) = (key, references) {
            return Ok((key, references));
        }
    }
    Err(syn::Error::new(
        proc_macro2::Span::call_site(),
        "dbkit: #[belongs_to] requires key = <field> and references = <field>",
    ))
}

fn extract_lit_str(expr: &syn::Expr) -> Option<String> {
    if let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(lit),
        ..
    }) = expr
    {
        Some(lit.value())
    } else {
        None
    }
}

fn extract_ident(expr: &syn::Expr) -> Option<Ident> {
    if let syn::Expr::Path(path) = expr {
        path.path.get_ident().cloned()
    } else {
        None
    }
}

fn option_inner_type(ty: &Type) -> Option<Type> {
    let path = match ty {
        Type::Path(path) => path,
        _ => return None,
    };
    let segment = path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }
    let args = match &segment.arguments {
        syn::PathArguments::AngleBracketed(args) => args,
        _ => return None,
    };
    let inner = args.args.first()?;
    match inner {
        syn::GenericArgument::Type(inner_ty) => Some(inner_ty.clone()),
        _ => None,
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
                let parsed = syn::Attribute::parse_outer
                    .parse2(new_attr)
                    .expect("derive attr");
                kept.extend(parsed);
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

// (unused helper removed)

// (intentionally removed unused AnyState helpers)
