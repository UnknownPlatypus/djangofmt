use std::fmt::Debug;

/// A trait for lint violations.
///
/// This trait is implemented by structs that represent specific lint rules.
/// It separates the rule logic (the struct) from the rule metadata (the registry).
pub trait Violation: Debug {
    /// The message to be displayed to the user.
    fn message(&self) -> String;

    /// Optional: Format arguments for the message.
    fn formats(&self) -> Vec<String> {
        vec![]
    }
}
