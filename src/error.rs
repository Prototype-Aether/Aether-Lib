use std::fmt::{Debug, Display, Formatter, Result};

pub struct AetherError {
    pub code: u16,
    pub description: String,
    pub cause: Option<Box<AetherError>>,
}

impl AetherError {
    pub fn new(_code: u16, _description: String, _cause: Option<Box<AetherError>>) -> AetherError {
        AetherError {
            code: _code,
            description: String::from(_description),
            cause: _cause,
        }
    }
}

impl Display for AetherError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "E{}: {}", self.code, self.description)
    }
}

impl Debug for AetherError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self.cause {
            Some(ref error) => {
                write!(f, "{}\nCause: {:?}", self, *error)
            }
            None => {
                write!(f, "{}", self)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::error::AetherError;

    #[test]
    fn display_test() {
        let err = AetherError {
            code: 9001,
            description: String::from("Test error"),
            cause: None,
        };

        assert_eq!(format!("{}", err), "E9001: Test error");
    }

    #[test]

    fn debug_test() {
        let err1 = AetherError {
            code: 9002,
            description: String::from("Bottom level error"),
            cause: None,
        };
        let err2 = AetherError {
            code: 9023,
            description: String::from("Middle level error"),
            cause: Some(Box::new(err1)),
        };
        let err3 = AetherError {
            code: 9032,
            description: String::from("Top level error"),
            cause: Some(Box::new(err2)),
        };

        assert_eq!(format!("{:?}", err3), 
            "E9032: Top level error\nCause: E9023: Middle level error\nCause: E9002: Bottom level error");
    }
}
