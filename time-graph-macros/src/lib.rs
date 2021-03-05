//! A procedural macro attribute for instrumenting functions with
//! [`time-graph`].
//!
//! [`time-graph`] provides always-on profiling for your code, allowing to
//! record the execution time of functions, spans inside these functions and the
//! actual call graph at run-time. This crate provides the
//! [`#[instrument]`][instrument] procedural macro attribute.
//!
//! Note that this macro is also re-exported by the main `time-graph` crate.
//!
//!
//! ## Usage
//!
//! First, add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! time-graph-macros = "0.1.0"
//! ```
//!
//! The [`#[instrument]`][instrument] attribute can now be added to a function
//! to automatically create a `time-graph` [callsite], and enter the
//! corresponding [span] when that function is called. For example:
//!
//! ```
//! use time_graph_macros::instrument;
//!
//! #[instrument]
//! pub fn my_function(my_arg: usize) {
//!     // ...
//! }
//!
//! # fn main() {}
//! ```
//!
//! [`time-graph`]: https://crates.io/crates/time-graph
//! [instrument]: macro@instrument
//! [callsite]: https://docs.rs/time-graph/latest/time_graph/struct.CallSite.html
//! [span]: https://docs.rs/time-graph/latest/time_graph/struct.Span.html

#![allow(clippy::needless_return)]

extern crate proc_macro;
use proc_macro::TokenStream;

use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ItemFn, Signature, LitStr, Token};


/// Instruments a function to create and enter a [`time-graph`] [span] every
/// time the function is called.
///
/// # Examples
/// Instrumenting a function:
/// ```
/// # use time_graph_macros::instrument;
/// #[instrument]
/// pub fn my_function(my_arg: usize) {
///     // ...
/// }
///
/// ```
/// Overriding the generated span's name:
/// ```
/// # use time_graph_macros::instrument;
/// #[instrument(name = "another name")]
/// pub fn my_function() {
///     // ...
/// }
/// ```
///
/// [span]: https://docs.rs/time-graph/latest/time_graph/struct.Span.html
/// [`time-graph`]: https://github.com/luthaf/time-graph
#[proc_macro_attribute]
pub fn instrument(args: TokenStream, tokens: TokenStream) -> TokenStream {
    let input: ItemFn = syn::parse_macro_input!(tokens as ItemFn);
    let args: TimedArgs = syn::parse_macro_input!(args as TimedArgs);

    let name = args.name.unwrap_or_else(|| input.sig.ident.to_string());

    let ItemFn {
        attrs,
        vis,
        block,
        sig,
        ..
    } = input;

    let Signature {
        output: return_type,
        inputs: params,
        unsafety,
        asyncness,
        constness,
        abi,
        ident,
        generics:
            syn::Generics {
                params: gen_params,
                where_clause,
                ..
            },
        ..
    } = sig;

    let stream = quote!(
        #(#attrs) *
        #vis #constness #unsafety #asyncness #abi fn #ident<#gen_params>(#params) #return_type
        #where_clause
        {
            time_graph::spanned!(#name, {
                #block
            })
        }
    );

    return stream.into();
}

struct TimedArgs {
    name: Option<String>,
}

mod kw {
    syn::custom_keyword!(name);
}

impl Parse for TimedArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut args = TimedArgs {
            name: None,
        };
        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::name) {
                if args.name.is_some() {
                    return Err(input.error("expected only a single `name` argument"));
                }
                let _ = input.parse::<kw::name>()?;
                let _ = input.parse::<Token![=]>()?;
                args.name = Some(input.parse::<LitStr>()?.value());
            } else if lookahead.peek(LitStr) {
                if args.name.is_some() {
                    return Err(input.error("expected only a single `name` argument"));
                }
                args.name = Some(input.parse::<LitStr>()?.value());
            } else {
                return Err(lookahead.error());
            }
        }
        Ok(args)
    }
}
