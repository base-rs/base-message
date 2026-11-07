// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::combination::{MultipleValues, SingleValue};
use crate::{FieldError, FieldType, InvalidValue, split_list, trim_whitespace};

/// Accumulates the instances of one declared field, as the derive's generated code does: `Option<T>` for a singleton field type, and `Vec<T>` for a list or separate one.
#[doc(hidden)]
pub trait Accumulate: Sized {
    type State: Default;

    /// The field type's invalid-value policy, used unless the declaration overrides it.
    const INVALID: InvalidValue;

    fn add(
        state: &mut Self::State,
        value: &[u8],
        position: usize,
        name: &'static str,
        invalid: InvalidValue,
    ) -> Result<(), FieldError>;

    fn finish(state: Self::State) -> Self;
}

/// Parse one value, applying the invalid-value policy.
fn parse<T: FieldType>(
    value: &[u8],
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

impl<T: FieldType> Accumulate for Option<T>
where
    T::Combination: SingleValue,
{
    /// The position of the first instance, which counts even if its value was ignored, and its value.
    type State = (Option<usize>, Option<T>);

    const INVALID: InvalidValue = T::INVALID;

    fn add(
        state: &mut Self::State,
        value: &[u8],
        position: usize,
        name: &'static str,
        invalid: InvalidValue,
    ) -> Result<(), FieldError> {
        if let Some(first) = state.0 {
            return Err(FieldError::duplicate(name, position, first));
        }
        state.0 = Some(position);
        state.1 = parse(value, position, name, invalid)?;
        Ok(())
    }

    fn finish(state: Self::State) -> Self {
        state.1
    }
}

impl<T: FieldType> Accumulate for Vec<T>
where
    T::Combination: MultipleValues,
{
    type State = Vec<T>;

    const INVALID: InvalidValue = T::INVALID;

    fn add(
        state: &mut Self::State,
        value: &[u8],
        position: usize,
        name: &'static str,
        invalid: InvalidValue,
    ) -> Result<(), FieldError> {
        if !<T::Combination as MultipleValues>::LIST {
            state.extend(parse(value, position, name, invalid)?);
            return Ok(());
        }
        let elements = match split_list(value) {
            Ok(elements) => elements,
            Err(_) if invalid == InvalidValue::Ignore => return Ok(()),
            Err(error) => return Err(FieldError::invalid(name, position, error)),
        };
        for element in elements {
            state.extend(parse(element, position, name, invalid)?);
        }
        Ok(())
    }

    fn finish(state: Self::State) -> Self {
        state
    }
}
