//! The Django/Jinja template model shared by the formatter and the linter.

mod comment;
mod diagnostic;
pub mod dtl;
mod ignore;
mod parse;
mod parse_error;
mod profile;

pub use comment::{CommentDelimiters, HTML_COMMENT, TEMPLATE_COMMENT, strip_bom};
pub use diagnostic::{clamp_offset, graphical_handler, span};
pub use ignore::{
    FILE_IGNORE, FORMAT_IGNORE_DIRECTIVES, FileIgnores, IGNORE, IGNORE_DIRECTIVE, IgnoreDirective,
    LEGACY_IGNORE_DIRECTIVE, NAMESPACE, ReservedCode,
};
pub use parse::{Parsed, parse};
pub use parse_error::{ParseError, SKIP_FILE_HINT};
pub use profile::Profile;
