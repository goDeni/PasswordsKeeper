#[derive(Clone, Debug)]
pub struct InputState {
    pub prompt: String,
    pub buffer: String,
    pub password_mode: bool,
}

impl InputState {
    pub fn new(prompt: impl Into<String>, password_mode: bool) -> Self {
        Self {
            prompt: prompt.into(),
            buffer: String::new(),
            password_mode,
        }
    }

    pub fn push_char(&mut self, c: char) {
        self.buffer.push(c);
    }

    pub fn backspace(&mut self) {
        self.buffer.pop();
    }

    pub fn take(&mut self) -> String {
        std::mem::take(&mut self.buffer)
    }

    pub fn display(&self) -> String {
        if self.password_mode {
            "*".repeat(self.buffer.len())
        } else {
            self.buffer.clone()
        }
    }
}
