pub struct AetherError {
    pub code: u16,
    pub description: String,
    pub cause: Option<Box<AetherError>>,
}

impl AetherError {
    pub fn traceback(&self) -> String {
        let mut result: String = String::new();

        result = result
            + &format!(
                "Error code: {}\n{}\nCaused by -\n",
                self.code, self.description
            );

        match self.cause {
            Some(ref error) => result += &error.traceback(),
            None => (),
        }

        result
    }

    pub fn print(&self) {
        println!("Traceback:\n{}", self.traceback());
    }
}
