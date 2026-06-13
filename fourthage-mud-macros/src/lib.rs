use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Meta, Lit};

/// Derive macro for `ComponentStorage` that generates standard HashMap-based implementations.
/// 
/// Requires a `#[component(field = "field_name")]` attribute specifying the HashMap field
/// in `EntityRegistryInternal`.
/// 
/// # Example
/// ```ignore
/// #[derive(ComponentStorage)]
/// #[component(field = "names")]
/// pub struct Name(String);
/// ```
#[proc_macro_derive(ComponentStorage, attributes(component))]
pub fn derive_component_storage(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let field_name = match extract_field_name(&input.attrs) {
        Ok(s) => s,
        Err(_) => {
            return syn::Error::new_spanned(
                name,
                "ComponentStorage derive requires #[component(field = \"field_name\")] attribute",
            )
            .to_compile_error().into();
        }
    };

    let field_ident = syn::Ident::new(&field_name, name.span());

    let expanded = quote! {
        impl ComponentStorage for #name {
            fn get<'a>(entities: &'a EntityRegistryInternal, entity: &EntityId) -> Option<&'a Self>
            where
                Self: Sized,
            {
                entities.#field_ident.get(entity)
            }

            fn update(entities: &mut EntityRegistryInternal, entity: &EntityId, component: Self)
            where
                Self: Sized,
            {
                entities.#field_ident.insert(entity.clone(), component);
            }

            fn remove(entities: &mut EntityRegistryInternal, entity: &EntityId)
            where
                Self: Sized,
            {
                entities.#field_ident.remove(entity);
            }

            fn storage(entities: &EntityRegistryInternal) -> &HashMap<EntityId, Self>
            where
                Self: Sized,
            {
                &entities.#field_ident
            }
        }
    };

    TokenStream::from(expanded)
}

fn extract_field_name(attrs: &[syn::Attribute]) -> Result<String, ()> {
    for attr in attrs {
        if attr.path().is_ident("component") {
            // Parse #[component(field = "name")]
            if let Meta::List(meta_list) = &attr.meta {
                // Parse the content of the list as a MetaNameValue
                let content: syn::MetaNameValue = syn::parse2(meta_list.tokens.clone()).map_err(|_| ())?;
                if content.path.is_ident("field") {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit_str),
                        ..
                    }) = &content.value
                    {
                        return Ok(lit_str.value());
                    }
                }
            }
        }
    }
    Err(())
}
