// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! The derive macro for `base-message` field sections. Use it through `base_message::FieldSection`, rather than depending on this crate directly.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use std::collections::BTreeMap;
use syn::parse::ParseStream;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Error, Ident, LitStr, Token, Type};

/// Derive the interpretation of a field section from a struct's declarations.
///
/// Each field of the struct is declared with `#[field("name")]`, naming a field in lowercase, or `#[fields(flatten)]`, embedding another section. A declared field's type says how its instances combine: `Option<T>` for an optional singleton, `T` for a required one, and `Vec<T>` for a list or separate field. `#[field("name", invalid = "ignore")]` overrides the field type's invalid-value policy. On the struct, `#[fields(forbid("name", ...))]` forbids fields, and `#[fields(unknown = "reject")]` rejects fields it neither declares nor forbids, which are otherwise ignored.
#[proc_macro_derive(FieldSection, attributes(field, fields))]
pub fn derive_field_section(input: TokenStream) -> TokenStream {
    // A derive's input always parses as an item, so errors come from the declarations.
    syn::parse::<DeriveInput>(input)
        .and_then(|input| expand(&input))
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// The struct-level declarations.
struct Section {
    rejects_unknown: bool,
    forbidden: Vec<LitStr>,
}

/// Whether a declared field may be absent, must be present, or collects several values.
enum Container {
    Optional,
    Required,
    Many,
}

struct Declared {
    member: Ident,
    ty: Type,
    name: LitStr,
    invalid: Option<Ident>,
    container: Container,
}

impl Declared {
    /// The accumulator type: `Option<T>` for a required `T`, or the declared container.
    fn accumulator(&self) -> TokenStream2 {
        let ty = &self.ty;
        match self.container {
            Container::Required => quote!(::core::option::Option<#ty>),
            Container::Optional | Container::Many => quote!(#ty),
        }
    }

    /// The generated call that adds one instance, spanned on the declared type so that type errors point at it.
    fn add(&self) -> TokenStream2 {
        let (member, name, accumulator) = (&self.member, &self.name, self.accumulator());
        let invalid = match &self.invalid {
            Some(invalid) => quote!(::base_message::InvalidValue::#invalid),
            None => quote!(<#accumulator as ::base_message::Accumulate>::INVALID),
        };
        quote_spanned!(self.ty.span() => <#accumulator as ::base_message::Accumulate>::add(&mut builder.#member, value, position, #name, #invalid)?)
    }
}

fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let Data::Struct(data) = &input.data else {
        return Err(Error::new_spanned(
            input,
            "FieldSection can only be derived for a struct with named fields",
        ));
    };
    let section = struct_section(input)?;
    let mut declared: Vec<Declared> = Vec::new();
    let mut flattened: Vec<(Ident, Type)> = Vec::new();
    for field in &data.fields {
        let member = field.ident.clone().ok_or_else(|| {
            Error::new_spanned(
                field,
                "FieldSection can only be derived for a struct with named fields",
            )
        })?;
        let mut kind = None;
        for attribute in &field.attrs {
            if attribute.path().is_ident("field") {
                let (name, invalid) = attribute.parse_args_with(parse_field)?;
                check_name(&name)?;
                if declared
                    .iter()
                    .any(|existing| existing.name.value() == name.value())
                {
                    return Err(Error::new_spanned(
                        &name,
                        "this field name is already declared",
                    ));
                }
                kind = Some(Some((name, invalid)));
            } else if attribute.path().is_ident("fields") {
                let word: Ident = attribute.parse_args()?;
                if word == "other" {
                    return Err(Error::new_spanned(
                        word,
                        "sections do not keep unknown fields; use `partition` to collect the fields a section does not take",
                    ));
                }
                if word != "flatten" {
                    return Err(Error::new_spanned(word, "expected `flatten`"));
                }
                flattened.push((member.clone(), field.ty.clone()));
                kind = Some(None);
            }
        }
        match kind {
            Some(Some((name, invalid))) => declared.push(Declared {
                container: container(&field.ty),
                member,
                ty: field.ty.clone(),
                name,
                invalid,
            }),
            Some(None) => {}
            None => {
                return Err(Error::new_spanned(
                    field,
                    "declare this field with `#[field(\"name\")]` or `#[fields(flatten)]`",
                ));
            }
        }
    }

    let ident = &input.ident;
    let visibility = &input.vis;
    let builder = quote::format_ident!("__{}FieldSectionBuilder", ident);

    let builder_fields = declared.iter().map(|declared| {
        let (member, accumulator) = (&declared.member, declared.accumulator());
        quote_spanned!(declared.ty.span() => #member: <#accumulator as ::base_message::Accumulate>::State)
    });
    let flattened_builders = flattened
        .iter()
        .map(|(member, ty)| quote!(#member: <#ty as ::base_message::FieldSection>::Builder));

    // Name dispatch jumps on the name's length and lowercased first byte, then compares the few names that share both. Forbidden names join the same jump, so refusing them costs nothing extra.
    let mut groups: BTreeMap<(usize, u8), Vec<TokenStream2>> = BTreeMap::new();
    for declared in &declared {
        let (name, add) = (&declared.name, declared.add());
        groups.entry(group(name)).or_default().push(quote! {
            if name.eq_ignore_ascii_case(#name) {
                #add;
                return ::core::result::Result::Ok(true);
            }
        });
    }
    for forbidden in &section.forbidden {
        if declared
            .iter()
            .any(|declared| declared.name.value() == forbidden.value())
        {
            return Err(Error::new_spanned(
                forbidden,
                "this field name is declared, so it cannot also be forbidden",
            ));
        }
        groups.entry(group(forbidden)).or_default().push(quote! {
            if name.eq_ignore_ascii_case(#forbidden) {
                return ::core::result::Result::Err(::base_message::FieldError::forbidden(#forbidden, position));
            }
        });
    }
    let arms = groups
        .iter()
        .map(|((length, first), checks)| quote!((#length, #first) => { #(#checks)* }));
    let flattened_accepts = flattened.iter().map(|(member, ty)| {
        quote! {
            if <#ty as ::base_message::FieldSection>::accept(&mut builder.#member, name, value, position)? {
                return ::core::result::Result::Ok(true);
            }
        }
    });

    // Declaration ids number this section's own declarations in order, then each flattened section's.
    let own_count = declared.len();
    let id_arms = declared.iter().enumerate().map(|(id, declared)| {
        let add = declared.add();
        quote!(#id => { #add; ::core::result::Result::Ok(()) })
    });
    let flattened_ids = flattened.iter().map(|(member, ty)| {
        quote! {
            if declaration < <#ty as ::base_message::FieldSection>::NAMES.count() {
                return <#ty as ::base_message::FieldSection>::accept_declaration(&mut builder.#member, declaration, value, position);
            }
            let declaration = declaration - <#ty as ::base_message::FieldSection>::NAMES.count();
        }
    });

    let finishes = declared.iter().map(|declared| {
        let (member, name, accumulator) = (&declared.member, &declared.name, declared.accumulator());
        match declared.container {
            Container::Required => quote!(#member: <#accumulator as ::base_message::Accumulate>::finish(builder.#member).ok_or_else(|| ::base_message::FieldError::missing(#name, end))?),
            Container::Optional | Container::Many => quote!(#member: <#accumulator as ::base_message::Accumulate>::finish(builder.#member)),
        }
    });
    let flattened_finishes = flattened.iter().map(|(member, ty)| quote!(#member: <#ty as ::base_message::FieldSection>::finish(builder.#member, end)?));
    let names = declared.iter().map(|declared| &declared.name);
    let forbidden = &section.forbidden;
    let flattened_names = flattened
        .iter()
        .map(|(_, ty)| quote!(&<#ty as ::base_message::FieldSection>::NAMES));
    let rejects_unknown = section.rejects_unknown;
    let conflict_check = quote_spanned! {ident.span() =>
        const _: () = ::core::assert!(!<#ident as ::base_message::FieldSection>::NAMES.conflicts(), "a field name is declared or forbidden by both this section and a section it flattens, so the two would disagree about it");
    };

    Ok(quote! {
        #[doc(hidden)]
        #[derive(Default)]
        #visibility struct #builder {
            #(#builder_fields,)*
            #(#flattened_builders,)*
        }

        impl ::base_message::FieldSection for #ident {
            const NAMES: ::base_message::FieldNames = ::base_message::FieldNames { own: &[#(#names),*], forbidden: &[#(#forbidden),*], flattened: &[#(#flattened_names),*] };

            const REJECTS_UNKNOWN: bool = #rejects_unknown;

            type Builder = #builder;

            #[inline]
            fn accept(builder: &mut Self::Builder, name: &str, value: &[u8], position: usize) -> ::core::result::Result<bool, ::base_message::FieldError> {
                if let ::core::option::Option::Some(&first) = name.as_bytes().first() {
                    match (name.len(), first.to_ascii_lowercase()) {
                        #(#arms)*
                        _ => {}
                    }
                }
                #(#flattened_accepts)*
                let _ = (builder, value, position);
                ::core::result::Result::Ok(false)
            }

            #[inline]
            fn accept_declaration(builder: &mut Self::Builder, declaration: usize, value: &[u8], position: usize) -> ::core::result::Result<(), ::base_message::FieldError> {
                match declaration {
                    #(#id_arms)*
                    _ => {
                        let declaration = declaration - #own_count;
                        #(#flattened_ids)*
                        let _ = (builder, value, declaration);
                        ::core::result::Result::Err(::base_message::FieldError::declaration(position))
                    }
                }
            }

            fn finish(builder: Self::Builder, end: usize) -> ::core::result::Result<Self, ::base_message::FieldError> {
                ::core::result::Result::Ok(Self {
                    #(#finishes,)*
                    #(#flattened_finishes,)*
                })
            }
        }

        #conflict_check
    })
}

/// A name's dispatch group: its length and first byte. Names are lowercase, so the first byte needs no conversion.
fn group(name: &LitStr) -> (usize, u8) {
    let text = name.value();
    (text.len(), text.as_bytes()[0])
}

fn parse_field(input: ParseStream) -> syn::Result<(LitStr, Option<Ident>)> {
    let name: LitStr = input.parse()?;
    let mut invalid = None;
    while !input.is_empty() {
        input.parse::<Token![,]>()?;
        let key: Ident = input.parse()?;
        if key != "invalid" {
            return Err(Error::new_spanned(
                key,
                "expected `invalid = \"error\"` or `invalid = \"ignore\"`",
            ));
        }
        input.parse::<Token![=]>()?;
        let value: LitStr = input.parse()?;
        invalid = Some(match value.value().as_str() {
            "error" => Ident::new("Error", value.span()),
            "ignore" => Ident::new("Ignore", value.span()),
            _ => {
                return Err(Error::new_spanned(
                    value,
                    "`invalid` must be \"error\" or \"ignore\"",
                ));
            }
        });
    }
    Ok((name, invalid))
}

/// The struct-level `#[fields(unknown = "reject" | "ignore", forbid("name", ...))]` declarations; ignoring unknown fields is the default.
fn struct_section(input: &DeriveInput) -> syn::Result<Section> {
    let mut section = Section {
        rejects_unknown: false,
        forbidden: Vec::new(),
    };
    for attribute in input
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("fields"))
    {
        attribute.parse_args_with(|stream: ParseStream| {
            while !stream.is_empty() {
                let key: Ident = stream.parse()?;
                if key == "unknown" {
                    stream.parse::<Token![=]>()?;
                    let value: LitStr = stream.parse()?;
                    section.rejects_unknown = match value.value().as_str() {
                        "reject" => true,
                        "ignore" => false,
                        _ => return Err(Error::new_spanned(value, "`unknown` must be \"reject\" or \"ignore\"; use `partition` to collect the fields a section does not take")),
                    };
                } else if key == "forbid" {
                    let content;
                    syn::parenthesized!(content in stream);
                    for name in content.parse_terminated(|input: ParseStream| input.parse::<LitStr>(), Token![,])? {
                        check_name(&name)?;
                        if section.forbidden.iter().any(|existing| existing.value() == name.value()) {
                            return Err(Error::new_spanned(&name, "this field name is already forbidden"));
                        }
                        section.forbidden.push(name);
                    }
                } else {
                    return Err(Error::new_spanned(key, "expected `unknown = \"...\"` or `forbid(\"name\", ...)`"));
                }
                if !stream.is_empty() {
                    stream.parse::<Token![,]>()?;
                }
            }
            Ok(())
        })?;
    }
    Ok(section)
}

/// Field names are matched case-insensitively, so they are declared once, in lowercase, as tokens.
fn check_name(name: &LitStr) -> syn::Result<()> {
    let text = name.value();
    let valid = !text.is_empty()
        && text.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"!#$%&'*+-.^_`|~".contains(&byte)
        });
    if valid {
        Ok(())
    } else {
        Err(Error::new_spanned(
            name,
            "a field name must be a lowercase token, such as \"content-type\"",
        ))
    }
}

/// `Option<T>` may be absent and `Vec<T>` collects values; any other type is a required singleton.
fn container(ty: &Type) -> Container {
    let Type::Path(path) = ty else {
        return Container::Required;
    };
    match path
        .path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
        .as_deref()
    {
        Some("Option") => Container::Optional,
        Some("Vec") => Container::Many,
        _ => Container::Required,
    }
}
