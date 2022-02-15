use std::fmt::{Debug, Display, Formatter, Result};

pub struct AetherError {
    pub code: u16,
    pub description: &'static str,
}

impl AetherError {
    pub fn new(code: u16, description: &'static str) -> AetherError {
        AetherError { code, description }
    }
}

impl Display for AetherError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "E{}: {}", self.code, self.description)
    }
}

impl Debug for AetherError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self)
    }
}

#[cfg(test)]
mod tests {
    use crate::error::AetherError;

    #[test]
    fn display_test() {
        let err = AetherError {
            code: 9001,
            description: "Test error",
        };

        assert_eq!(format!("{}", err), "E9001: Test error");
    }

    #[test]

    fn debug_test() {
        let err1 = AetherError {
            code: 9002,
            description: "Some Error",
        };
        
        assert_eq!(format!("{:?}", err1), "E9002: Some Error");
        // assert_eq!(format!("{:?}", err3), 
            // "E9032: Top level error\nCause: E9023: Middle level error\nCause: E9002: Bottom level error");
    }
}
