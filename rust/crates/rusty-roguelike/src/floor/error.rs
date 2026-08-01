use std::{error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FloorAdmissionError {
    code: String,
    detail: String,
}

impl FloorAdmissionError {
    pub(crate) fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for FloorAdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.detail)
    }
}

impl Error for FloorAdmissionError {}
