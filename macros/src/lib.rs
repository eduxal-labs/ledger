use argon2::{Algorithm, Argon2, Params, Version};
use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemEnum, parse_macro_input};

const M_COST: u32 = 19456;
const T_COST: u32 = 8;
const P_COST: u32 = 4;
const OUTPUT_LEN: usize = 32;

#[proc_macro]
pub fn key(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::LitStr).value();
    let value = match std::env::var(&input) {
        Ok(value) => value,
        Err(_) => return quote! {compile_error!("environment variable not found")}.into(),
    };
    let pwd = value.as_bytes();
    let salt = b"PASETO_KEY_PASSWORD";
    let argon2 = argon2();
    let mut key = [0u8; OUTPUT_LEN];
    if let Ok(()) = argon2.hash_password_into(pwd, salt, &mut key) {
        return quote! { const KEY: [u8; #OUTPUT_LEN] = [#(#key),*]; }.into();
    }
    quote! { compile_error!("could not hash the password to generate the key") }.into()
}

fn argon2() -> Argon2<'static> {
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params())
}

fn params() -> Params {
    Params::new(M_COST, T_COST, P_COST, Some(OUTPUT_LEN)).unwrap()
}

#[proc_macro_derive(Count)]
pub fn count(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemEnum);
    let name = &input.ident;
    let count = input.variants.len();
    let variants = input
        .variants
        .iter()
        .map(|variant| &variant.ident)
        .collect::<Vec<_>>();
    quote! {
        impl #name {
            pub const COUNT: usize = #count;
            pub const VARIANTS: [Self; #count] = [#(Self::#variants),*];
        }
    }
    .into()
}
