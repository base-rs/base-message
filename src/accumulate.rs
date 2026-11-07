// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::combination::{MultipleValues, SingleValue};
use crate::{FieldError, FieldType, InvalidValue, split_list, trim_whitespace};

/// The builder's storage for one declared field: its accumulated value, and the position of its first instance.
///
/// It places no bound on `A`, so that a builder is a valid type even when a declaration is not, and a mismatched declaration reports one error, at its type, rather than one for each use of the builder.
pub struct Slot<A> {
    first: Option<usize>,
    value: A,
}

impl<A: Default> Default for Slot<A> {
    fn default() -> Self {
        Self {
            first: None,
            value: A::default(),
        }
    }
}

/// Accumulates the instances of one declared field, as the derive's generated code does: `Option<T>` for a singleton field type, and `Vec<T>` for a list or separate one.
pub trait Accumulate<'a>: Sized {
    /// The field type's invalid-value policy, used unless the declaration overrides it.
    const INVALID: InvalidValue;

    /// Add one instance at `position`, applying the invalid-value policy `invalid`.
    fn add(
        slot: &mut Slot<Self>,
        value: &'a [u8],
        position: usize,
        name: &'static str,
        invalid: InvalidValue,
    ) -> Result<(), FieldError>;

    /// The accumulated value.
    fn finish(slot: Slot<Self>) -> Self {
        slot.value
    }
}

/// Parse one value, applying the invalid-value policy.
fn parse<'a, T: FieldType<'a>>(
    value: &'a [u8],
    position: usize,
    name: &'static str,
    invalid: InvalidValue,
) -> Result<Option<T>, FieldError> {
    match T::parse(trim_whitespace(value)) {
        Ok(value) => Ok(Some(value)),
        Err(_) if invalid == InvalidValue::Ignore => Ok(None),
        Err(error) => Err(FieldError::invalid(name, position, error)),
    }
}

impl<'a, T: FieldType<'a>> Accumulate<'a> for Option<T>
where
    T::Combination: SingleValue,
{
    const INVALID: InvalidValue = T::INVALID;

    /// The first instance counts as present even if its value was ignored, so a second instance is still a duplicate.
    fn add(
        slot: &mut Slot<Self>,
        value: &'a [u8],
        position: usize,
        name: &'static str,
        invalid: InvalidValue,
    ) -> Result<(), FieldError> {
        if let Some(first) = slot.first {
            return Err(FieldError::duplicate(name, position, first));
        }
        slot.first = Some(position);
        slot.value = parse(value, position, name, invalid)?;
        Ok(())
    }
}

impl<'a, T: FieldType<'a>> Accumulate<'a> for Vec<T>
where
    T::Combination: MultipleValues,
{
    const INVALID: InvalidValue = T::INVALID;

    fn add(
        slot: &mut Slot<Self>,
        value: &'a [u8],
        position: usize,
        name: &'static str,
        invalid: InvalidValue,
    ) -> Result<(), FieldError> {
        if !<T::Combination as MultipleValues>::LIST {
            slot.value.extend(parse(value, position, name, invalid)?);
            return Ok(());
        }
        // An instance whose list does not split is skipped as a whole under `Ignore`, so the elements it added before the error are removed.
        let start = slot.value.len();
        for element in split_list(value) {
            let element = match element {
                Ok(element) => element,
                Err(_) if invalid == InvalidValue::Ignore => {
                    slot.value.truncate(start);
                    return Ok(());
                }
                Err(error) => return Err(FieldError::invalid(name, position, error)),
            };
            slot.value.extend(parse(element, position, name, invalid)?);
        }
        Ok(())
    }
}
