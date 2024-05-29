use quote::quote;
use syn::Item;

pub fn derive_export_type(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let s = syn::parse::<Item>(input).unwrap();

    let (target_struct, fields) = match s {
        Item::Struct(ref s) => (
            s.ident.clone(),
            s.fields
                .iter()
                .map(|field| field.ty.clone())
                .collect::<Vec<_>>(),
        ),
        Item::Enum(ref e) => (
            e.ident.clone(),
            e.variants
                .iter()
                .flat_map(|variant| variant.fields.iter())
                .map(|field| field.ty.clone())
                .collect::<Vec<_>>(),
        ),
        _ => unimplemented!(),
    };

    quote! {
        impl qubit::ExportType for #target_struct {
            fn export(registry: &mut qubit::builder::ty::TypeRegistry) {
                // Insert this type
                let exists = registry.register(<Self as ts_rs::TS>::name(), <Self as ts_rs::TS>::inline());

                if exists {
                    return;
                }

                // Insert field types
                #(<#fields as qubit::ExportType>::export(registry);)*
            }
        }
    }
    .into()
}
