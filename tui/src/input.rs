#[derive(Clone, Debug)]
pub struct InputState {
    pub prompt: String,
    pub buffer: String,
    pub password_mode: bool,
    pub password_visible: bool,
}

impl InputState {
    pub fn new(prompt: impl Into<String>, password_mode: bool) -> Self {
        Self {
            prompt: prompt.into(),
            buffer: String::new(),
            password_mode,
            password_visible: false,
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
        if self.password_mode && !self.password_visible {
            "*".repeat(self.buffer.len())
        } else {
            self.buffer.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InputState;

    #[test]
    fn test_input_state_new_plain() {
        let input = InputState::new("Prompt", false);
        assert_eq!(input.prompt, "Prompt");
        assert_eq!(input.buffer, "");
        assert!(!input.password_mode);
        assert!(!input.password_visible);
    }

    #[test]
    fn test_input_state_new_password() {
        let input = InputState::new("Password", true);
        assert_eq!(input.prompt, "Password");
        assert!(input.password_mode);
        assert!(!input.password_visible);
    }

    #[test]
    fn test_push_char_and_backspace() {
        let mut input = InputState::new("Prompt", false);
        input.push_char('a');
        input.push_char('b');
        assert_eq!(input.buffer, "ab");

        input.backspace();
        assert_eq!(input.buffer, "a");

        input.backspace();
        input.backspace();
        assert_eq!(input.buffer, "");
    }

    #[test]
    fn test_take_clears_buffer() {
        let mut input = InputState::new("Prompt", false);
        input.push_char('x');
        input.push_char('y');
        let value = input.take();
        assert_eq!(value, "xy");
        assert_eq!(input.buffer, "");
    }

    #[test]
    fn test_display_plain_mode() {
        let mut input = InputState::new("Prompt", false);
        input.push_char('a');
        input.push_char('b');
        input.push_char('c');

        assert_eq!(input.display(), "abc");
    }

    #[test]
    fn test_display_password_mode_hidden() {
        let mut input = InputState::new("Prompt", true);
        input.push_char('a');
        input.push_char('b');
        input.push_char('c');

        assert_eq!(input.display(), "***");
    }

    #[test]
    fn test_display_password_mode_visible() {
        let mut input = InputState::new("Prompt", true);
        input.push_char('a');
        input.push_char('b');
        input.push_char('c');
        input.password_visible = true;

        assert_eq!(input.display(), "abc");
    }
}
