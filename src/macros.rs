//! This module provides two utility macros for testing custom derives. They can
//! be used together to eliminate some of the boilerplate required in order to
//! declare and test custom derive implementations.

// Re-exports used by the decl_derive! and test_derive!
pub use proc_macro::TokenStream;
pub use proc_macro2::TokenStream as TokenStream2;
pub use syn::{parse, parse_str, DeriveInput};

/// The `decl_derive!` macro declares a custom derive wrapper. It will parse the
/// incoming `TokenStream` into a `synstructure::Structure` object, and pass it
/// into the inner function.
///
/// Your inner function should take a `synstructure::Structure` by value, and
/// return a type implementing `synstructure::MacroResult`, for example:
///
/// ```
/// fn derive_simple(input: synstructure::Structure) -> proc_macro2::TokenStream {
///     unimplemented!()
/// }
///
/// fn derive_result(input: synstructure::Structure)
///     -> syn::Result<proc_macro2::TokenStream>
/// {
///     unimplemented!()
/// }
/// ```
///
/// # Usage
///
/// ### Without Attributes
/// ```
/// fn derive_interesting(_input: synstructure::Structure) -> proc_macro2::TokenStream {
///     quote::quote! { ... }
/// }
///
/// # const _IGNORE: &'static str = stringify! {
/// decl_derive!([Interesting] => derive_interesting);
/// # };
/// ```
///
/// ### With Attributes
/// ```
/// # fn main() {}
/// fn derive_interesting(_input: synstructure::Structure) -> proc_macro2::TokenStream {
///     quote::quote! { ... }
/// }
///
/// # const _IGNORE: &'static str = stringify! {
/// decl_derive!([Interesting, attributes(interesting_ignore)] => derive_interesting);
/// # };
/// ```
#[macro_export]
macro_rules! decl_derive {
    // XXX: Switch to using this variant everywhere?
    ([$derives:ident $($derive_t:tt)*] => $inner:path) => {
        #[proc_macro_derive($derives $($derive_t)*)]
        #[allow(non_snake_case)]
        pub fn $derives(
            i: $crate::macros::TokenStream
        ) -> $crate::macros::TokenStream {
            match $crate::macros::parse::<$crate::macros::DeriveInput>(i) {
                Ok(p) => {
                    match $crate::Structure::try_new(&p) {
                        Ok(s) => $crate::MacroResult::into_stream($inner(s)),
                        Err(e) => e.to_compile_error().into(),
                    }
                }
                Err(e) => e.to_compile_error().into(),
            }
        }
    };
}

/// The `decl_attribute!` macro declares a custom attribute wrapper. It will
/// parse the incoming `TokenStream` into a `synstructure::Structure` object,
/// and pass it into the inner function.
///
/// Your inner function should have the following type:
///
/// ```
/// fn attribute(
///     attr: proc_macro2::TokenStream,
///     structure: synstructure::Structure,
/// ) -> proc_macro2::TokenStream {
///     unimplemented!()
/// }
/// ```
///
/// # Usage
///
/// ```
/// fn attribute_interesting(
///     _attr: proc_macro2::TokenStream,
///     _structure: synstructure::Structure,
/// ) -> proc_macro2::TokenStream {
///     quote::quote! { ... }
/// }
///
/// # const _IGNORE: &'static str = stringify! {
/// decl_attribute!([interesting] => attribute_interesting);
/// # };
/// ```
/// ```
#[macro_export]
macro_rules! decl_attribute {
    ([$attribute:ident] => $inner:path) => {
        #[proc_macro_attribute]
        pub fn $attribute(
            attr: $crate::macros::TokenStream,
            i: $crate::macros::TokenStream,
        ) -> $crate::macros::TokenStream {
            match $crate::macros::parse::<$crate::macros::DeriveInput>(i) {
                Ok(p) => match $crate::Structure::try_new(&p) {
                    Ok(s) => $crate::MacroResult::into_stream($inner(attr.into(), s)),
                    Err(e) => e.to_compile_error().into(),
                },
                Err(e) => e.to_compile_error().into(),
            }
        }
    };
}

/// Run a test on a custom derive. This macro expands both the original struct
/// and the expansion to ensure that they compile correctly, and confirms that
/// feeding the original struct into the named derive will produce the written
/// output.
///
/// You can add `no_build` to the end of the macro invocation to disable
/// checking that the written code compiles. This is useful in contexts where
/// the procedural macro cannot depend on the crate where it is used during
/// tests.
///
/// # Usage
///
/// ```
/// fn test_derive_example(_s: synstructure::Structure)
///     -> Result<proc_macro2::TokenStream, syn::Error>
/// {
///     Ok(quote::quote! { const YOUR_OUTPUT: &'static str = "here"; })
/// }
///
/// fn main() {
///     synstructure::test_derive!{
///         test_derive_example {
///             struct A;
///         }
///         expands to {
///             const YOUR_OUTPUT: &'static str = "here";
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! test_derive {
    ($name:path { $($i:tt)* } expands to { $($o:tt)* }) => {
        {
            #[allow(dead_code)]
            fn ensure_compiles() {
                $($i)*
                $($o)*
            }

            $crate::test_derive!($name { $($i)* } expands to { $($o)* } no_build);
        }
    };

    ($name:path { $($i:tt)* } expands to { $($o:tt)* } no_build) => {
        {
            let i = stringify!( $($i)* );
            let parsed = $crate::macros::parse_str::<$crate::macros::DeriveInput>(i)
                .expect(concat!(
                    "Failed to parse input to `#[derive(",
                    stringify!($name),
                    ")]`",
                ));

            let raw_res = $name($crate::Structure::new(&parsed));
            let res = $crate::MacroResult::into_result(raw_res)
                .expect(concat!(
                    "Procedural macro failed for `#[derive(",
                    stringify!($name),
                    ")]`",
                ));

            let expected = stringify!( $($o)* )
                .parse::<$crate::macros::TokenStream2>()
                .expect("output should be a valid TokenStream");
            let mut expected_toks = $crate::macros::TokenStream2::from(expected);
            if res.to_string() != expected_toks.to_string() {
                panic!("\
test_derive failed:
expected:
```
{}
```

got:
```
{}
```\n",
                    $crate::unpretty_print(&expected_toks),
                    $crate::unpretty_print(&res),
                );
            }
        }
    };
}

/// A helper macro for declaring relatively straightforward derive
/// implementations. It provides mechanisms for operating over structures
/// performing modifications on each field etc.
///
/// This macro doesn't define the actual derive, but rather the implementation
/// method. Use `decl_derive!` to generate the implementation wrapper.
///
/// # Stability Warning
///
/// This is an unstable experimental macro API, which may be changed or removed
/// in a future version. I'm not yet confident enough that this API is useful
/// enough to warrant its complexity and inclusion in `synstructure`.
///
/// # Caveat
///
/// The `quote!` macro from `quote` must be imported in the calling crate, as
/// this macro depends on it.
///
/// # Note
///
/// This feature is implemented behind the `simple-derive` feature, and is only
/// available when that feature is enabled.
///
/// # Example
///
/// ```
/// # use quote::quote;
/// # const _IGNORE: &'static str = stringify! {
/// synstructure::decl_derive!([Interest] => derive_interest);
/// # };
///
/// synstructure::simple_derive! {
///     // This macro implements the `Interesting` method exported by the `aa`
///     // crate. It will explicitly add an `extern crate` invocation to import the
///     // crate into the expanded context.
///     derive_interest impl synstructure_test_traits::Interest {
///         // A "filter" block can be added. It evaluates its body with the (s)
///         // variable bound to a mutable reference to the input `Structure`
///         // object.
///         //
///         // This block can be used to perform general transformations, such as
///         // filtering out fields which should be ignored by all methods and for
///         // the purposes of binding type parameters.
///         filter(s) {
///             s.filter(|bi| bi.ast().ident != Some(syn::Ident::new("a",
///                 proc_macro2::Span::call_site())));
///         }
///
///         // This is an implementation of a method in the implemented crate. The
///         // return value should be the series of match patterns to destructure
///         // the `self` argument with.
///         fn interesting(&self as s) -> bool {
///             s.fold(false, |acc, bi| {
///                 quote!(#acc || synstructure_test_traits::Interest::interesting(#bi))
///             })
///         }
///     }
/// }
///
/// fn main() {
///     synstructure::test_derive!{
///         derive_interest {
///             struct A<T> {
///                 x: i32,
///                 a: bool, // Will be ignored by filter
///                 c: T,
///             }
///         }
///         expands to {
///             #[allow(non_upper_case_globals)]
///             #[doc(hidden)]
///             const _DERIVE_synstructure_test_traits_Interest_FOR_A: () = {
///                 extern crate synstructure_test_traits;
///                 impl<T> synstructure_test_traits::Interest for A<T>
///                     where T: synstructure_test_traits::Interest
///                 {
///                     fn interesting(&self) -> bool {
///                         match *self {
///                             A {
///                                 x: ref __binding_0,
///                                 c: ref __binding_2,
///                                 ..
///                             } => {
///                                 false ||
///                                     synstructure_test_traits::Interest::interesting(__binding_0) ||
///                                     synstructure_test_traits::Interest::interesting(__binding_2)
///                             }
///                         }
///                     }
///                 }
///             };
///         }
///     }
/// }
/// ```
#[cfg(feature = "simple-derive")]
#[macro_export]
macro_rules! simple_derive {
    // entry point
    (
        $iname:ident impl $path:path { $($rest:tt)* }
    ) => {
        $crate::simple_derive!(__I [$iname, $path] { $($rest)* } [] []);
    };

    // Adding a filter block
    (
        __I $opt:tt {
            filter($s:ident) {
                $($body:tt)*
            }
            $($rest:tt)*
        } [$($done:tt)*] [$($filter:tt)*]
    ) => {
        $crate::simple_derive!(
            __I $opt { $($rest)* } [$($done)*] [
                $($filter)*
                [
                    st_name = $s,
                    body = {
                        $($body)*
                    },
                ]
            ]
        );
    };

    // &self bound method
    (
        __I $opt:tt {
            fn $fn_name:ident (&self as $s:ident $($params:tt)*) $(-> $t:ty)* {
                $($body:tt)*
            }
            $($rest:tt)*
        } [$($done:tt)*] [$($filter:tt)*]
    ) => {
        $crate::simple_derive!(
            __I $opt { $($rest)* } [
                $($done)*
                [
                    st_name = $s,
                    bind_style = Ref,
                    body = { $($body)* },
                    result = result,
                    expanded = {
                        fn $fn_name(&self $($params)*) $(-> $t)* {
                            match *self { #result }
                        }
                    },
                ]
            ] [$($filter)*]
        );
    };

    // &mut self bound method
    (
        __I $opt:tt {
            fn $fn_name:ident (&mut self as $s:ident $($params:tt)*) $(-> $t:ty)* {
                $($body:tt)*
            }
            $($rest:tt)*
        } [$($done:tt)*] [$($filter:tt)*]
    ) => {
        $crate::simple_derive!(
            __I $opt { $($rest)* } [
                $($done)*
                [
                    st_name = $s,
                    bind_style = RefMut,
                    body = { $($body)* },
                    result = result,
                    expanded = {
                        fn $fn_name(&mut self $($params)*) $(-> $t)* {
                            match *self { #result }
                        }
                    },
                ]
            ] [$($filter)*]
        );
    };

    // self bound method
    (
        __I $opt:tt {
            fn $fn_name:ident (self as $s:ident $($params:tt)*) $(-> $t:ty)* {
                $($body:tt)*
            }
            $($rest:tt)*
        } [$($done:tt)*] [$($filter:tt)*]
    ) => {
        $crate::simple_derive!(
            __I $opt { $($rest)* } [
                $($done)*
                [
                    st_name = $s,
                    bind_style = Move,
                    body = { $($body)* },
                    result = result,
                    expanded = {
                        fn $fn_name(self $($params)*) $(-> $t)* {
                            match self { #result }
                        }
                    },
                ]
            ] [$($filter)*]
        );
    };

    // XXX: Static methods?

    // codegen after data collection
    (
        __I [$iname:ident, $path:path] {} [$(
            [
                st_name = $st_name:ident,
                bind_style = $bind_style:ident,
                body = $body:tt,
                result = $result:ident,
                expanded = { $($expanded:tt)* },
            ]
        )*] [$(
            [
                st_name = $filter_st_name:ident,
                body = $filter_body:tt,
            ]
        )*]
    ) => {
        fn $iname(mut st: $crate::Structure) -> $crate::macros::TokenStream2 {
            let _ = &mut st; // Silence the unused mut warning

            // Filter/transform the `Structure` object before cloning it for
            // individual methods.
            $(
                {
                    let $filter_st_name = &mut st;
                    $filter_body
                }
            )*

            // Clone the `Structure` object and set the correct binding style,
            // then perform method specific expansion.
            $(
                let $result = {
                    let mut $st_name = st.clone();
                    $st_name.bind_with(|_| $crate::BindStyle::$bind_style);
                    let $result = {
                        $body
                    };
                    $crate::quote!{ $($expanded)* }
                };
            )*

            st.bound_impl($crate::quote!($path), $crate::quote!{
                $(#$result)*
            })
        }
    }
}
