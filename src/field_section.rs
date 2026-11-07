// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::__private::IncomingName;
use crate::field_ref::IntoField;
use crate::{FieldError, FieldNames, FieldRef};

/// A field section interpreted into a struct. It is implemented only by `#[derive(FieldSection)]`.
///
/// A section takes the fields it declares, refuses those it forbids, and either ignores or rejects the rest. [`Self::interpret`] discards the fields it does not take; [`Self::partition`] returns them, borrowed and with their original positions, for another section to interpret.
///
/// The lifetime is that of the field values, which a section with a lifetime parameter can borrow; a section that owns its values implements `FieldSection<'a>` for every `'a`.
pub trait FieldSection<'a>: Sized {
    /// The names this section declares and forbids, including those of sections it flattens.
    const NAMES: FieldNames;

    /// Whether the section rejects fields it neither declares nor forbids, including because a section it flattens does.
    #[doc(hidden)]
    const REJECTS_UNKNOWN: bool = Self::NAMES.rejects_unknown();

    /// The section's type with its lifetime, if any, as `'static`, which identifies the section to its static tables.
    #[doc(hidden)]
    type Identity: 'static;

    #[doc(hidden)]
    type Builder;

    /// An empty builder, before any field is accepted.
    #[doc(hidden)]
    fn builder() -> Self::Builder;

    /// Offer one field by name, reporting whether a declaration took it. A forbidden name is an error.
    #[doc(hidden)]
    fn accept(
        builder: &mut Self::Builder,
        name: &str,
        value: &'a [u8],
        position: usize,
    ) -> Result<bool, FieldError>;

    /// Accept a field by its declaration id, as [`FieldNames::declaration`] numbers them, without comparing names.
    #[doc(hidden)]
    fn accept_declaration(
        builder: &mut Self::Builder,
        declaration: usize,
        value: &'a [u8],
        position: usize,
    ) -> Result<(), FieldError>;

    /// Build the section, reporting a missing required field.
    #[doc(hidden)]
    fn finish(builder: Self::Builder) -> Result<Self, FieldError>;

    /// Interpret a section from its fields in wire order, taking those it declares and discarding the rest.
    ///
    /// # Errors
    ///
    /// The first [`FieldError`]: a duplicate singleton, an invalid value its field treats as an error, a forbidden field, an unknown field the section rejects, or a missing required field.
    fn interpret<I>(fields: I) -> Result<Self, FieldError>
    where
        I: IntoIterator,
        I::Item: IntoField<'a, Self>,
    {
        interpret::<Self, I, Vec<FieldRef<'a>>>(fields, None)
    }

    /// Interpret a section from its fields in wire order, taking those it declares and returning the rest, with their original positions, for another section to interpret.
    ///
    /// The remainder can be any collection: a `Vec<FieldRef>` borrows the fields without copying them, and [`OwnedFields`](crate::OwnedFields) copies them. A field a static table classified as undeclared for this section is passed on by name, since the next section may declare it. A forbidden field is an error, never part of the remainder.
    ///
    /// # Errors
    ///
    /// As for [`Self::interpret`].
    fn partition<I, R>(fields: I) -> Result<(Self, R), FieldError>
    where
        I: IntoIterator,
        I::Item: IntoField<'a, Self>,
        R: Default + Extend<FieldRef<'a>>,
    {
        let mut remainder = R::default();
        let section = interpret::<Self, I, R>(fields, Some(&mut remainder))?;
        Ok((section, remainder))
    }
}

fn interpret<'a, S, I, R>(fields: I, mut remainder: Option<&mut R>) -> Result<S, FieldError>
where
    S: FieldSection<'a>,
    I: IntoIterator,
    I::Item: IntoField<'a, S>,
    R: Extend<FieldRef<'a>>,
{
    let mut builder = S::builder();
    for (index, field) in fields.into_iter().enumerate() {
        let field = field.into_field(index);
        let unclaimed = match field.name {
            IncomingName::Declaration(declaration) => {
                S::accept_declaration(
                    &mut builder,
                    usize::from(declaration),
                    field.value,
                    field.position,
                )?;
                None
            }
            IncomingName::Forbidden(name) => {
                return Err(FieldError::forbidden(name, field.position));
            }
            IncomingName::Undeclared(name) => Some(name),
            IncomingName::Text(name) => {
                (!S::accept(&mut builder, name, field.value, field.position)?).then_some(name)
            }
        };
        if let Some(name) = unclaimed {
            if S::REJECTS_UNKNOWN {
                return Err(FieldError::unknown(field.position));
            }
            if let Some(remainder) = remainder.as_deref_mut() {
                remainder.extend([FieldRef::new(field.position, name, field.value)]);
            }
        }
    }
    S::finish(builder)
}
