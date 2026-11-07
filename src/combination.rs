// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! How the instances of a field combine, as a type-level marker on each [`FieldType`](crate::FieldType).
//!
//! The marker decides how a section may declare the field: a [`Singleton`] as `Option<T>`, or as `T` when it is required, and a [`List`] or [`Separate`] field as `Vec<T>`. Declaring one the wrong way is a compile-time error.

/// A field that appears at most once, such as `Content-Length` or `Content-Type`: a second instance is an error, even with the same value.
pub struct Singleton;

/// A field whose instances form one comma-separated list, as RFC 9110 section 5.3 combines them, such as `Accept` or `Cache-Control`.
pub struct List;

/// A field whose instances are separate values that must not be combined, because a value can contain a comma, such as `Set-Cookie` or an email's `Received`.
pub struct Separate;

/// The combination of a field that appears at most once: [`Singleton`].
#[diagnostic::on_unimplemented(
    message = "a field type that can repeat cannot be declared as `Option<T>` or `T`",
    label = "instances of this field type combine into several values",
    note = "declare a list or separate field as `Vec<T>`, so that every value is kept"
)]
pub trait SingleValue {}

impl SingleValue for Singleton {}

/// The combinations that collect several values: [`List`] and [`Separate`].
#[diagnostic::on_unimplemented(
    message = "a singleton field type cannot be declared as `Vec<T>`",
    label = "this field type appears at most once",
    note = "declare a singleton field as `Option<T>`, or as `T` when it is required"
)]
pub trait MultipleValues {
    /// Whether one instance holds a comma-separated list of elements.
    const LIST: bool;
}

impl MultipleValues for List {
    const LIST: bool = true;
}

impl MultipleValues for Separate {
    const LIST: bool = false;
}
