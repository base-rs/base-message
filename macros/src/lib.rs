// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! The derive macro for `base-message` field sections. Use it through `base_message::FieldSection`, rather than depending on this crate directly.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use std::collections::BTreeMap;
use syn::parse::ParseStream;
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{Data, DeriveInput, Error, GenericParam, Ident, Lifetime, LitStr, Token, Type};

/// Derive the interpretation of a field section from a struct's declarations.
///
/// Each member of the struct is declared with `#[field("name")]`, naming a field in lowercase, or `#[fields(flatten)]`, embedding another section. A declared member's type says how its instances combine: `Option<T>` for an optional singleton, `T` for a required one, and `Vec<T>` for a list or separate field. `#[field("name", invalid = "ignore")]` overrides the field type's invalid-value policy, except that a required field always treats an invalid value as an error. On the struct, `#[fields(forbid("name", ...))]` forbids fields, and `#[fields(unknown = "reject")]` rejects fields it neither declares nor forbids, which are otherwise ignored. A section may have one lifetime parameter, for values it borrows from the input.
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
    /// `Some(true)` for `unknown = "reject"`, `Some(false)` for an explicit `unknown = "ignore"`, with the setting's literal.
    unknown: Option<(bool, LitStr)>,
    forbidden: Vec<LitStr>,
}

/// Whether a declared member may be absent, must be present, or collects several values.
#[derive(PartialEq)]
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

struct Flattened {
    member: Ident,
    ty: Type,
    /// The type with the section's lifetime as `'static`, for checks evaluated outside the impl.
    identity: Type,
}

/// What one member declares.
enum Member {
    Declared(LitStr, Option<Ident>),
    Flattened,
}

impl Declared {
    /// The accumulator type: `Option<T>` for a required `T`, or the declared container, spanned on the declared type so that type errors point at it.
    fn accumulator(&self) -> TokenStream2 {
        let ty = &self.ty;
        match self.container {
            Container::Required => quote_spanned!(ty.span() => ::core::option::Option<#ty>),
            Container::Optional | Container::Many => quote!(#ty),
        }
    }

    /// The generated call that adds one instance, spanned on the declared type so that type errors point at it.
    fn add(&self, lifetime: &Lifetime) -> TokenStream2 {
        let (member, name, accumulator) = (&self.member, &self.name, self.accumulator());
        let invalid = match (&self.invalid, &self.container) {
            (_, Container::Required) => quote!(::base_message::InvalidValue::Error),
            (Some(invalid), _) => quote!(::base_message::InvalidValue::#invalid),
            (None, _) => {
                quote!(<#accumulator as ::base_message::__private::Accumulate<#lifetime>>::INVALID)
            }
        };
        quote_spanned!(self.ty.span() => <#accumulator as ::base_message::__private::Accumulate<#lifetime>>::add(&mut __builder.#member, __value, __position, #name, #invalid)?)
    }
}

/// Replaces every lifetime with `'static`. In a member's type, the only lifetime other than `'static` that can appear is the section's own.
struct StaticLifetimes;

impl VisitMut for StaticLifetimes {
    fn visit_lifetime_mut(&mut self, lifetime: &mut Lifetime) {
        *lifetime = Lifetime::new("'static", lifetime.span());
    }
}

fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let Data::Struct(data) = &input.data else {
        return Err(Error::new_spanned(
            input,
            "FieldSection can only be derived for a struct with named fields",
        ));
    };
    let own_lifetime = section_lifetime(input)?;
    let lifetime = own_lifetime
        .clone()
        .unwrap_or_else(|| Lifetime::new("'__field", Span::call_site()));
    let to_static = |ty: &Type| {
        let mut ty = ty.clone();
        StaticLifetimes.visit_type_mut(&mut ty);
        ty
    };
    let section = struct_section(input)?;
    let mut declared: Vec<Declared> = Vec::new();
    let mut flattened: Vec<Flattened> = Vec::new();
    for field in &data.fields {
        let member = field.ident.clone().ok_or_else(|| {
            Error::new_spanned(
                field,
                "FieldSection can only be derived for a struct with named fields",
            )
        })?;
        let mut kind = None;
        for attribute in &field.attrs {
            let is_field = attribute.path().is_ident("field");
            if !is_field && !attribute.path().is_ident("fields") {
                continue;
            }
            if kind.is_some() {
                return Err(Error::new_spanned(
                    attribute,
                    "a member takes one `#[field(\"name\")]` or `#[fields(flatten)]` declaration",
                ));
            }
            kind = Some(if is_field {
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
                Member::Declared(name, invalid)
            } else {
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
                Member::Flattened
            });
        }
        match kind {
            Some(Member::Declared(name, invalid)) => {
                let container = container(&field.ty);
                if let (Container::Required, Some(invalid)) = (&container, &invalid) {
                    if invalid == "Ignore" {
                        return Err(Error::new(
                            invalid.span(),
                            "a required field always treats an invalid value as an error; declare it as `Option<T>` to make it optional",
                        ));
                    }
                }
                declared.push(Declared {
                    container,
                    member,
                    ty: field.ty.clone(),
                    name,
                    invalid,
                });
            }
            Some(Member::Flattened) => flattened.push(Flattened {
                member,
                identity: to_static(&field.ty),
                ty: field.ty.clone(),
            }),
            None => {
                return Err(Error::new_spanned(
                    field,
                    "declare this member with `#[field(\"name\")]` or `#[fields(flatten)]`",
                ));
            }
        }
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
    }

    let ident = &input.ident;
    let (section_type, identity) = match &own_lifetime {
        Some(own) => (quote!(#ident<#own>), quote!(#ident<'static>)),
        None => (quote!(#ident), quote!(#ident)),
    };

    let builder_fields = declared.iter().map(|declared| {
        let (member, accumulator) = (&declared.member, declared.accumulator());
        quote!(#member: ::base_message::__private::Slot<#accumulator>)
    });
    let flattened_builders = flattened.iter().map(|Flattened { member, ty, .. }| quote!(#member: <#ty as ::base_message::FieldSection<#lifetime>>::Builder));
    let builder_members = declared.iter().map(|declared| &declared.member);
    let flattened_members = flattened.iter().map(|Flattened { member, ty, .. }| quote!(#member: <#ty as ::base_message::FieldSection<#lifetime>>::builder()));

    // Name dispatch jumps on the name's length and lowercased first byte, then compares the few names that share both. Forbidden names join the same jump, so refusing them costs nothing extra.
    let mut groups: BTreeMap<(usize, u8), Vec<TokenStream2>> = BTreeMap::new();
    for declared in &declared {
        let (name, add) = (&declared.name, declared.add(&lifetime));
        groups.entry(group(name)).or_default().push(quote! {
            if __name.eq_ignore_ascii_case(#name) {
                #add;
                return ::core::result::Result::Ok(true);
            }
        });
    }
    for forbidden in &section.forbidden {
        groups.entry(group(forbidden)).or_default().push(quote! {
            if __name.eq_ignore_ascii_case(#forbidden) {
                return ::core::result::Result::Err(::base_message::__private::forbidden(#forbidden, __position));
            }
        });
    }
    let arms = groups
        .iter()
        .map(|((length, first), checks)| quote!((#length, #first) => { #(#checks)* }));
    let flattened_accepts = flattened.iter().map(|Flattened { member, ty, .. }| {
        quote! {
            if <#ty as ::base_message::FieldSection<#lifetime>>::accept(&mut __builder.#member, __name, __value, __position)? {
                return ::core::result::Result::Ok(true);
            }
        }
    });

    let accept_declaration = accept_declaration(&declared, &flattened, &lifetime);

    let finishes = declared.iter().map(|declared| {
        let (member, name, accumulator) = (&declared.member, &declared.name, declared.accumulator());
        let finish = quote_spanned!(declared.ty.span() => <#accumulator as ::base_message::__private::Accumulate<#lifetime>>::finish(__builder.#member));
        match declared.container {
            Container::Required => quote!(#member: #finish.ok_or_else(|| ::base_message::__private::missing(#name))?),
            Container::Optional | Container::Many => quote!(#member: #finish),
        }
    });
    let flattened_finishes = flattened.iter().map(|Flattened { member, ty, .. }| quote!(#member: <#ty as ::base_message::FieldSection<#lifetime>>::finish(__builder.#member)?));
    let names = declared.iter().map(|declared| &declared.name);
    let forbidden = &section.forbidden;
    let flattened_names = flattened.iter().map(|Flattened { identity, .. }| quote!(&<#identity as ::base_message::FieldSection<'static>>::NAMES));
    let rejects_unknown = matches!(section.unknown, Some((true, _)));
    let checks = checks(&declared, &flattened, &section);

    Ok(quote! {
        const _: () = {
            #[doc(hidden)]
            pub struct __FieldSectionBuilder<#lifetime> {
                #(#builder_fields,)*
                #(#flattened_builders,)*
                __lifetime: ::core::marker::PhantomData<&#lifetime ()>,
            }

            #[automatically_derived]
            impl<#lifetime> ::base_message::FieldSection<#lifetime> for #section_type {
                const NAMES: ::base_message::FieldNames = ::base_message::FieldNames::new(&[#(#names),*], &[#(#forbidden),*], &[#(#flattened_names),*], #rejects_unknown);

                type Identity = #identity;

                type Builder = __FieldSectionBuilder<#lifetime>;

                // Not `Default`, which requires the builder to be `Sized`, so that a mismatched declaration's error refers to this crate's requirements rather than the standard library's.
                fn builder() -> Self::Builder {
                    __FieldSectionBuilder {
                        #(#builder_members: ::core::default::Default::default(),)*
                        #(#flattened_members,)*
                        __lifetime: ::core::marker::PhantomData,
                    }
                }

                #[inline]
                fn accept(__builder: &mut Self::Builder, __name: &str, __value: &#lifetime [u8], __position: usize) -> ::core::result::Result<bool, ::base_message::FieldError> {
                    if let ::core::option::Option::Some(&__first) = __name.as_bytes().first() {
                        match (__name.len(), __first.to_ascii_lowercase()) {
                            #(#arms)*
                            _ => {}
                        }
                    }
                    #(#flattened_accepts)*
                    ::core::result::Result::Ok(false)
                }

                #[inline]
                fn accept_declaration(__builder: &mut Self::Builder, __declaration: usize, __value: &#lifetime [u8], __position: usize) -> ::core::result::Result<(), ::base_message::FieldError> {
                    #accept_declaration
                }

                fn finish(__builder: Self::Builder) -> ::core::result::Result<Self, ::base_message::FieldError> {
                    ::core::result::Result::Ok(Self {
                        #(#finishes,)*
                        #(#flattened_finishes,)*
                    })
                }
            }

            #checks
        };
    })
}

/// The body of `accept_declaration`. Declaration ids number this section's own declarations in order, then each flattened section's.
///
/// A static table classifies only a section's own declarations, so an id the section does not have means a broken invariant, such as a hand-written `FieldSection` whose identity names another section; it panics rather than giving the value to another declaration.
fn accept_declaration(
    declared: &[Declared],
    flattened: &[Flattened],
    lifetime: &Lifetime,
) -> TokenStream2 {
    let own_count = declared.len();
    let arms = declared.iter().enumerate().map(|(id, declared)| {
        let add = declared.add(lifetime);
        quote!(#id => { #add; ::core::result::Result::Ok(()) })
    });
    let routes = flattened.iter().map(|Flattened { member, ty, identity }| {
        quote! {
            let __count = const { <#identity as ::base_message::FieldSection<'static>>::NAMES.count() };
            if __declaration < __count {
                return <#ty as ::base_message::FieldSection<#lifetime>>::accept_declaration(&mut __builder.#member, __declaration, __value, __position);
            }
            let __declaration = __declaration - __count;
        }
    });
    quote! {
        match __declaration {
            #(#arms)*
            _ => {
                let __declaration = __declaration - #own_count;
                #(#routes)*
                ::core::unreachable!("a static table classified a declaration id past this section's declarations")
            }
        }
    }
}

/// Compile-time checks: no name is declared or forbidden both by this section and by a section it flattens, or by two flattened sections, and an explicit `unknown = "ignore"` does not contradict a flattened section that rejects unknown fields.
fn checks(declared: &[Declared], flattened: &[Flattened], section: &Section) -> TokenStream2 {
    let mut checks = Vec::new();
    let names = declared
        .iter()
        .map(|declared| &declared.name)
        .chain(&section.forbidden);
    for name in names {
        for Flattened {
            member, identity, ..
        } in flattened
        {
            let message = format!(
                "the field name {:?} is also declared or forbidden by the section flattened as `{member}`, so the two would disagree about it",
                name.value()
            );
            checks.push(quote_spanned! {name.span() =>
                const _: () = ::core::assert!(!<#identity as ::base_message::FieldSection<'static>>::NAMES.names(#name), #message);
            });
        }
    }
    for (index, second) in flattened.iter().enumerate() {
        for first in &flattened[..index] {
            let message = format!(
                "the sections flattened as `{}` and `{}` both declare or forbid a field name, so they would disagree about it",
                first.member, second.member
            );
            let (left, right) = (&first.identity, &second.identity);
            checks.push(quote_spanned! {second.member.span() =>
                const _: () = ::core::assert!(!<#left as ::base_message::FieldSection<'static>>::NAMES.shares_names_with(&<#right as ::base_message::FieldSection<'static>>::NAMES), #message);
            });
        }
    }
    if let Some((false, setting)) = &section.unknown {
        for Flattened {
            member, identity, ..
        } in flattened
        {
            let message = format!(
                "the section flattened as `{member}` rejects unknown fields, so this section cannot ignore them"
            );
            checks.push(quote_spanned! {setting.span() =>
                const _: () = ::core::assert!(!<#identity as ::base_message::FieldSection<'static>>::NAMES.rejects_unknown(), #message);
            });
        }
    }
    quote!(#(#checks)*)
}

/// A name's dispatch group: its length and first byte. Names are lowercase, so the first byte needs no conversion.
fn group(name: &LitStr) -> (usize, u8) {
    let text = name.value();
    (text.len(), text.as_bytes()[0])
}

/// The section's lifetime parameter, if it has one. A section may borrow from its input through one lifetime, and has no type or const parameters, which the fields' interpretation could not choose.
fn section_lifetime(input: &DeriveInput) -> syn::Result<Option<Lifetime>> {
    let generics = &input.generics;
    let message = "a field section may have one lifetime parameter, for values it borrows from the input, and no other generic parameters, bounds, or where clause";
    if generics.params.len() > 1 || generics.where_clause.is_some() {
        return Err(Error::new_spanned(generics, message));
    }
    match generics.params.first() {
        None => Ok(None),
        Some(GenericParam::Lifetime(parameter))
            if parameter.bounds.is_empty() && parameter.attrs.is_empty() =>
        {
            Ok(Some(parameter.lifetime.clone()))
        }
        Some(parameter) => Err(Error::new_spanned(parameter, message)),
    }
}

fn parse_field(input: ParseStream) -> syn::Result<(LitStr, Option<Ident>)> {
    let name: LitStr = input.parse()?;
    let mut invalid = None;
    while !input.is_empty() {
        input.parse::<Token![,]>()?;
        if input.is_empty() {
            break;
        }
        let key: Ident = input.parse()?;
        if key != "invalid" {
            return Err(Error::new_spanned(
                key,
                "expected `invalid = \"error\"` or `invalid = \"ignore\"`",
            ));
        }
        if invalid.is_some() {
            return Err(Error::new_spanned(key, "`invalid` is already given"));
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
        unknown: None,
        forbidden: Vec::new(),
    };
    for attribute in &input.attrs {
        if attribute.path().is_ident("field") {
            return Err(Error::new_spanned(
                attribute,
                "`#[field(...)]` declares a member; on the struct, use `#[fields(...)]`",
            ));
        }
        if !attribute.path().is_ident("fields") {
            continue;
        }
        attribute.parse_args_with(|stream: ParseStream| {
            while !stream.is_empty() {
                let key: Ident = stream.parse()?;
                if key == "unknown" {
                    if section.unknown.is_some() {
                        return Err(Error::new_spanned(key, "`unknown` is already given"));
                    }
                    stream.parse::<Token![=]>()?;
                    let value: LitStr = stream.parse()?;
                    let rejects = match value.value().as_str() {
                        "reject" => true,
                        "ignore" => false,
                        _ => return Err(Error::new_spanned(value, "`unknown` must be \"reject\" or \"ignore\"; use `partition` to collect the fields a section does not take")),
                    };
                    section.unknown = Some((rejects, value));
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
