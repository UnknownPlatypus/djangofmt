//! Type definitions for HTML element and attribute specifications.

/// Specification for an HTML element.
#[derive(Debug, Clone)]
pub struct ElementSpec {
    /// Element tag name (lowercase).
    pub name: &'static str,
    /// Whether the element is deprecated.
    pub deprecated: bool,
    /// Whether the element is a void element (no closing tag).
    pub void_element: bool,
    /// Element-specific attributes.
    pub attributes: &'static [AttributeSpec],
}

/// Specification for an attribute.
#[derive(Debug, Clone)]
pub struct AttributeSpec {
    /// Attribute name (lowercase).
    pub name: &'static str,
    /// Whether the attribute is deprecated.
    pub deprecated: bool,
    /// Type constraint for the attribute value.
    pub value_type: AttributeValueType,
}

/// Type constraint for attribute values.
#[derive(Debug, Clone)]
pub enum AttributeValueType {
    /// Any value allowed.
    Any,
    /// Boolean attribute (no value or empty string).
    Boolean,
    /// Enumerated values (case-insensitive).
    Enum(&'static [&'static str]),
    /// URL value.
    Url,
    /// Integer value.
    Integer,
    /// Positive integer value.
    PositiveInteger,
    /// Floating point value.
    Number,
}
