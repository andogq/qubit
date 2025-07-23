use proc_macro2::TokenStream;
use quote::quote;

pub fn ts(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ts_rs_path = quote!(::qubit::__private::ts_rs);
    let ts_rs_path_str = ts_rs_path.to_string();

    let attr = {
        let crate_attr = quote!(crate = #ts_rs_path_str);

        // Append any user-provided arguments
        if attr.is_empty() {
            crate_attr
        } else {
            quote!(#crate_attr, #attr)
        }
    };

    quote! {
        #[derive(#ts_rs_path::TS)]
        #[ts(#attr)]
        #item
    }
}
