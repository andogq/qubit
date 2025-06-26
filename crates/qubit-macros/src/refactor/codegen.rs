use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Type};

use crate::refactor::analyse::Implementation;

use super::lower::Ir;

pub fn codegen(ir: Ir) -> TokenStream {
    let Ir {
        name,
        visibility,
        ctx_ty,
        inner_ctx_ty,
        parse_params,
        handler_params,
        handler_return_ty,
        implementation:
            Implementation {
                block: impl_block,
                attrs: impl_attrs,
                asyncness: impl_asyncness,
                inputs: impl_inputs,
                output: impl_output,
            },
        register_method,
        register_params,
        qubit_types,
        handler_kind_str,
    } = ir;

    let ts_type = generate_ts_signature(
        &handler_kind_str,
        parse_params.as_deref(),
        &handler_return_ty,
    );

    // TODO: If `parse_params` isn't in `Option`, then this isn't needed.
    let parse_impl = parse_params.as_ref().map(|parse_params| {
        let (parse_params_idents, parse_params_tys): (Vec<_>, Vec<_>) =
            parse_params.iter().map(|(ident, ty)| (ident, ty)).unzip();

        quote! {
            let (#(#parse_params_idents,)*) = match params.parse::<(#(#parse_params_tys,)*)>() {
                ::std::result::Result::Ok(params) => params,
                ::std::result::Result::Err(e) => return ::std::result::Result::Err(e),
            };
        }
    });

    // TODO: Deal with this better
    let parse_params_tys = parse_params.iter().flatten().map(|(_, ty)| ty);

    quote! {
        #[allow(non_camel_case_types)]
        #visibility struct #name;
        impl<#inner_ctx_ty> ::qubit::Handler<#inner_ctx_ty> for #name
            where #inner_ctx_ty: 'static + ::std::marker::Send + ::std::marker::Sync + ::std::clone::Clone,
                #ctx_ty: ::qubit::FromRequestExtensions<#inner_ctx_ty>,
        {
            fn get_type() -> ::qubit::HandlerType {
                ::qubit::HandlerType {
                    name: ::std::stringify!(#name).to_string(),
                    signature: #ts_type,
                    kind: #handler_kind_str.to_string(),
                }
            }

            fn register(rpc_builder: ::qubit::RpcBuilder<#inner_ctx_ty>) -> ::qubit::RpcBuilder<#inner_ctx_ty> {
                #(#impl_attrs)*
                #impl_asyncness fn handler(#impl_inputs) #impl_output #impl_block

                rpc_builder.#register_method(
                    #(#register_params,)*
                    |ctx: #ctx_ty, params| async move {
                        #parse_impl

                        let result = handler(#(#handler_params),*).await;
                        ::std::result::Result::Ok::<_, ::qubit::ErrorObject>(result)
                    }
                )
            }

            fn export_all_dependencies_to(out_dir: &::std::path::Path) -> ::std::result::Result<::std::vec::Vec<::ts_rs::Dependency>, ::ts_rs::ExportError> {
                // Export the return type
                let mut dependencies = ::qubit::ty::util::export_with_dependencies::<#handler_return_ty>(out_dir)?;

                // Export each of the parameters
                #(dependencies.extend(::qubit::ty::util::export_with_dependencies::<#parse_params_tys>(out_dir)?);)*

                ::std::result::Result::Ok(dependencies)
            }

            fn qubit_types() -> ::std::vec::Vec<::qubit::ty::util::QubitType> {
                ::std::vec![#(#qubit_types),*]
            }
        }
    }
}

fn generate_ts_signature(
    handler_kind_str: impl AsRef<str>,
    params: Option<&[(Ident, Type)]>,
    return_ty: &Type,
) -> TokenStream {
    let handler_ty = handler_kind_str.as_ref();
    let params = params
        .map(|parse_params| {
            let fmt_str = "{}: {}, ".repeat(parse_params.len());
            let fmt_params = parse_params.iter().map(|(ident, ty)| {
                let ident = ident.to_string();
                quote! { #ident, <#ty as ::ts_rs::TS>::name() }
            });

            quote! {
                format!(#fmt_str, #(#fmt_params),*)
            }
        })
        .unwrap_or(quote! { "" });

    quote! {
        format!(
            "{handler_ty}<[{params}], {return_ty}>",
            handler_ty = #handler_ty,
            params = #params,
            return_ty = <#return_ty as ::ts_rs::TS>::name()
        )
    }
}
