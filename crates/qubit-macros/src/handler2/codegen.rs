use proc_macro2::TokenStream;
use quote::quote;

use super::lower::Ir;

pub fn codegen(ir: Ir) -> TokenStream {
    let Ir {
        name,
        kind,
        rpc_name,
        param_names,
        handler,
    } = ir;

    quote! {
        #handler

        const _: () = {
            #[::qubit::__private::linkme::distributed_slice(::qubit::__private::HANDLER_DEFINITIONS)]
            #[linkme(crate = ::qubit::__private::linkme)]
            static HANDLER_DEFINITION: fn() -> (::core::any::TypeId, ::qubit::__private::HandlerMeta) = || (
                ::core::any::Any::type_id(&#name),
                ::qubit::__private::HandlerMeta {
                    kind: #kind,
                    name: #rpc_name,
                    param_names: &[#(#param_names),*]
                }
            );
        };
    }
}
