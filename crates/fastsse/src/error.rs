use core::fmt;
use std::error::Error;

/// Errors raised while encoding SSE frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncodeError {
  field: &'static str,
  problem: &'static str,
}

impl EncodeError {
  pub(crate) const fn new(field: &'static str, problem: &'static str) -> Self {
    Self { field, problem }
  }

  /// Returns the field that failed validation.
  #[must_use]
  pub const fn field(self) -> &'static str {
    self.field
  }

  /// Returns the validation failure description.
  #[must_use]
  pub const fn problem(self) -> &'static str {
    self.problem
  }
}

impl fmt::Display for EncodeError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      formatter,
      "invalid {field} field: {problem}",
      field = self.field,
      problem = self.problem
    )
  }
}

impl Error for EncodeError {}

/// Errors raised while decoding SSE frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodeError {
  field: &'static str,
  problem: &'static str,
}

impl DecodeError {
  #[allow(dead_code)]
  pub(crate) const fn new(field: &'static str, problem: &'static str) -> Self {
    Self { field, problem }
  }

  /// Returns the field that failed decoding.
  #[must_use]
  pub const fn field(self) -> &'static str {
    self.field
  }

  /// Returns the decoding failure description.
  #[must_use]
  pub const fn problem(self) -> &'static str {
    self.problem
  }
}

impl fmt::Display for DecodeError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      formatter,
      "invalid {field} field while decoding SSE stream: {problem}",
      field = self.field,
      problem = self.problem
    )
  }
}

impl Error for DecodeError {}
