// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::{FieldError, FieldName, FieldNames, FieldRef, IntoFieldRef};

/// A field section interpreted into a struct, as `#[derive(FieldSection)]` generates.
///
/// A section takes the fields it declares, refuses those it forbids, and either ignores or rejects the rest. [`Self::interpret`] discards the fields it does not take; [`Self::partition`] returns them, borrowed and with their original positions, for another section to interpret.
pub trait FieldSection: Sized {
    /// The names this section declares and forbids, including those of sections it flattens.
    const NAMES: FieldNames;

    /// Whether the section rejects fields it neither declares nor forbids.
    #[doc(hidden)]
    const REJECTS_UNKNOWN: bool;

    #[doc(hidden)]
    type Builder: Default;

    /// Offer one field by name, reporting whether a declaration took it. A forbidden name is an error.
    #[doc(hidden)]
    fn accept(
        builder: &mut Self::Builder,
        name: &str,
        value: &[u8],
        position: usize,
    ) -> Result<bool, FieldError>;

    /// Accept a field by its declaration id, as [`FieldNames::declaration`] numbers them, without comparing names.
    #[doc(hidden)]
    fn accept_declaration(
        builder: &mut Self::Builder,
        declaration: usize,
        value: &[u8],
        position: usize,
    ) -> Result<(), FieldError>;

    /// Build the section, reporting a missing required field at `end`, just past the section's last field.
    #[doc(hidden)]
    fn finish(builder: Self::Builder, end: usize) -> Result<Self, FieldError>;

    /// Interpret a section from its fields in wire order, taking those it declares and discarding the rest.
    ///
    /// # Errors
    ///
    /// The first [`FieldError`]: a duplicate singleton, an invalid value its field treats as an error, a forbidden field, an unknown field the section rejects, or a missing required field.
    fn interpret<'a, I>(fields: I) -> Result<Self, FieldError>
    where
        I: IntoIterator,
        I::Item: IntoFieldRef<'a>,
    {
        interpret::<Self, I, Vec<FieldRef<'a>>>(fields, None)
    }

    /// Interpret a section from its fields in wire order, taking those it declares and returning the rest, with their original positions, for another section to interpret.
    ///
    /// The remainder can be any collection: a `Vec<FieldRef>` borrows the fields without copying them, and [`OwnedFields`](crate::OwnedFields) copies them. A field a static table classified as undeclared for this section is passed on as text, since the next section may declare it. A forbidden field is an error, never part of the remainder.
    ///
    /// # Errors
    ///
    /// As for [`Self::interpret`].
    fn partition<'a, I, R>(fields: I) -> Result<(Self, R), FieldError>
    where
        I: IntoIterator,
        I::Item: IntoFieldRef<'a>,
        R: Default + Extend<FieldRef<'a>>,
    {
        let mut remainder = R::default();
        let section = interpret::<Self, I, R>(fields, Some(&mut remainder))?;
        Ok((section, remainder))
    }
}

fn interpret<'a, S, I, R>(fields: I, mut remainder: Option<&mut R>) -> Result<S, FieldError>
where
    S: FieldSection,
    I: IntoIterator,
    I::Item: IntoFieldRef<'a>,
    R: Extend<FieldRef<'a>>,
{
    let mut builder = S::Builder::default();
    let mut end = 0;
    for (index, field) in fields.into_iter().enumerate() {
        let field = field.into_field_ref(index);
        end = end.max(field.position + 1);
        let unclaimed = match field.name {
            FieldName::Declaration(declaration, _) => {
                S::accept_declaration(
                    &mut builder,
                    usize::from(declaration),
                    field.value,
                    field.position,
                )?;
                None
            }
            FieldName::Forbidden(name) => {
                return Err(FieldError::forbidden(
                    S::NAMES
                        .forbidden_name(name)
                        .ok_or_else(|| FieldError::declaration(field.position))?,
                    field.position,
                ));
            }
            FieldName::Undeclared(name) => Some(name),
            FieldName::Text(name) => {
                (!S::accept(&mut builder, name, field.value, field.position)?).then_some(name)
            }
        };
        if let Some(name) = unclaimed {
            if S::REJECTS_UNKNOWN {
                return Err(FieldError::unknown(field.position));
            }
            if let Some(remainder) = remainder.as_deref_mut() {
                remainder.extend([FieldRef {
                    position: field.position,
                    name: FieldName::Text(name),
                    value: field.value,
                }]);
            }
        }
    }
    S::finish(builder, end)
}
