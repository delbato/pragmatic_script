extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;
extern crate quote;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{
    ItemEnum,
    Fields,
    Ident,
    Lit,
    Path
};
use quote::quote;

#[proc_macro_derive(TokenType, attributes(end, error, token, regex, skip))]
pub fn derive_token_type(input: TokenStream) -> TokenStream {
    let item: ItemEnum = syn::parse(input).expect("Only Enums can be used as a TokenType.");

    let enum_size = item.variants.len();
    let name = &item.ident;

    let mut match_statements: Vec<TokenStream2> = Vec::new();
    let mut skip_statements: Vec<TokenStream2> = Vec::new();
    let mut regex_init_statements: Vec<TokenStream2> = Vec::new();

    let token_attr_ident = syn::parse_str::<Ident>("token").unwrap();
    let regex_attr_ident = syn::parse_str::<Ident>("regex").unwrap();
    let skip_attr_ident = syn::parse_str::<Ident>("skip").unwrap();
    let error_attr_ident = syn::parse_str::<Ident>("error").unwrap();
    let end_attr_ident = syn::parse_str::<Ident>("end").unwrap();

    let mut end_set = false;
    let mut error_set = false;

    let mut err_accessor = syn::parse_str::<Ident>("error").unwrap();
    let mut end_accessor = syn::parse_str::<Ident>("end").unwrap();

    for variant in &item.variants {
        let variant_ident = &variant.ident;
        let accessor = format!("{}::{}", name, variant_ident);

        if variant.discriminant.is_some() {
            panic!("`{}::{}` has a discriminant, this is not allowed for a TokenType.", name, variant.ident);
        }
        match variant.fields {
            Fields::Unit => {},
            _ => panic!("`{}::{}` has fields, this is not allowed for a TokenType.", name, variant.ident),
        }

        for attr in &variant.attrs {
            let (attr_ident, attr_lit) = read_attribute(attr);
            
            // If this token variant is matched by a literal
            if attr_ident == token_attr_ident {
                if let Some(Lit::Str(literal)) = attr_lit {
                    let literal_value = literal.value();

                    let match_statement = quote! {
                        if input == #literal_value {
                            matches.push(#name::#variant_ident);
                        }
                    };

                    match_statements.push(match_statement);
                } else {
                    panic!("Value for token attribute must be a string literal.");
                }
            }
            // If this token variant is matched by a regex
            else if attr_ident == regex_attr_ident {
                if let Some(Lit::Str(literal)) = attr_lit {
                    let mut literal_value = literal.value();

                    literal_value.insert_str(0, "^");
                    literal_value += "$";

                    let regex_ident_string = format!("{}_regex", variant_ident);
                    let regex_ident = syn::parse_str::<Ident>(&regex_ident_string).expect("Unknown parse error.");

                    let regex_init_statement = quote! {
                        let #regex_ident = Regex::new(#literal_value).unwrap();
                    };
                    
                    let match_statement = quote! {
                        if #regex_ident.is_match(input) {
                            matches.push(#name::#variant_ident);
                        }
                    };

                    regex_init_statements.push(regex_init_statement);
                    match_statements.push(match_statement);
                } else {
                    panic!("Value for regex attribute must be a string literal.");
                }
            }
            else if attr_ident == end_attr_ident {
                if end_set {
                    panic!("Only one end variant can be defined for a TokenType.");
                }
                end_set = true;
                end_accessor = variant_ident.clone();
            }
            else if attr_ident == error_attr_ident {
                if error_set {
                    panic!("Only one error variant can be defined for a TokenType.");
                }
                error_set = true;
                err_accessor = variant_ident.clone();
            }
            // If this token variant should be skipped
            else if attr_ident == skip_attr_ident {
                let skip_statement = quote! {
                    if *self == #name::#variant_ident {
                        return true;
                    }
                };

                skip_statements.push(skip_statement);
            }
        }
    }

    if !end_set {
        panic!("You need to specify an end variant for a TokenType.");
    }
    if !error_set {
        panic!("You need to specify an error variant for a TokenType.");
    }

    let token_stream = quote! {
        impl TokenType for #name {
            fn lexer<'source, S>(source: S) -> Lexer<#name, S>
            where S: Source<'source> {
                let mut ret = Lexer::new(source);
                ret.advance();
                ret
            }

            fn match_token(input: &str) -> Vec<#name> {
                let mut matches: Vec<#name> = Vec::new();
                
                #(
                    #regex_init_statements
                )*

                #(
                    #match_statements
                )*

                matches
            }

            fn get_end_variant() -> #name {
                #name::#end_accessor
            }

            fn get_error_variant() -> #name {
                #name::#err_accessor
            }

            fn should_skip(&self) -> bool {
                #(
                    #skip_statements
                )*

                false
            }
        }
    };
    token_stream.into()
}

fn read_attribute(attr: &syn::Attribute) -> (Ident, Option<Lit>) {
    let meta = attr.parse_meta().expect("Attribute malformed: Meta parsing failed.");
    let ret = match meta {
        syn::Meta::NameValue(args) => {
            (args.path.get_ident().cloned().expect("Attribute malformed: Parsing of path to ident failed."), Some(args.lit))
        },
        syn::Meta::Path(path) => {
            (path.get_ident().cloned().expect("Attribute malformed: Parsing of path to ident failed."), None)
        },
        _ => panic!("Attribute malformed: Unknown attribute type.")
    };
    ret
}