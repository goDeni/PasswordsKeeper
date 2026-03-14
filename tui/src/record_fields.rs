#[derive(Clone, Debug)]
pub struct RecordFields {
    pub password: String,
    pub name: String,
    pub login: Option<String>,
    pub description: Option<String>,
}

impl RecordFields {
    pub fn new() -> Self {
        Self {
            password: String::new(),
            name: String::new(),
            login: None,
            description: None,
        }
    }

    pub fn is_password_complete(&self) -> bool {
        !self.password.is_empty()
    }

    pub fn is_name_complete(&self) -> bool {
        !self.name.is_empty()
    }

    pub fn get_current_step(&self) -> AddRecordStep {
        if !self.is_password_complete() {
            AddRecordStep::Password
        } else if !self.is_name_complete() {
            AddRecordStep::Name
        } else if self.login.is_none() {
            AddRecordStep::Login
        } else if self.description.is_none() {
            AddRecordStep::Description
        } else {
            AddRecordStep::Complete
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AddRecordStep {
    Password,
    Name,
    Login,
    Description,
    Complete,
}

#[cfg(test)]
mod tests {
    use super::{AddRecordStep, RecordFields};

    #[test]
    fn test_initial_step_is_password() {
        let fields = RecordFields::new();
        assert_eq!(fields.get_current_step(), AddRecordStep::Password);
    }

    #[test]
    fn test_step_name_after_password() {
        let mut fields = RecordFields::new();
        fields.password = "secret".to_string();
        assert_eq!(fields.get_current_step(), AddRecordStep::Name);
    }

    #[test]
    fn test_step_login_after_name() {
        let mut fields = RecordFields::new();
        fields.password = "secret".to_string();
        fields.name = "mail".to_string();
        assert_eq!(fields.get_current_step(), AddRecordStep::Login);
    }

    #[test]
    fn test_step_description_after_login_set() {
        let mut fields = RecordFields::new();
        fields.password = "secret".to_string();
        fields.name = "mail".to_string();
        fields.login = Some("user".to_string());
        assert_eq!(fields.get_current_step(), AddRecordStep::Description);
    }

    #[test]
    fn test_step_complete_after_description_set() {
        let mut fields = RecordFields::new();
        fields.password = "secret".to_string();
        fields.name = "mail".to_string();
        fields.login = Some("user".to_string());
        fields.description = Some("desc".to_string());
        assert_eq!(fields.get_current_step(), AddRecordStep::Complete);
    }

    #[test]
    fn test_empty_password_not_complete() {
        let mut fields = RecordFields::new();
        fields.password = "".to_string();
        assert!(!fields.is_password_complete());
        assert_eq!(fields.get_current_step(), AddRecordStep::Password);
    }

    #[test]
    fn test_empty_name_not_complete() {
        let mut fields = RecordFields::new();
        fields.password = "secret".to_string();
        fields.name = "".to_string();
        assert!(!fields.is_name_complete());
        assert_eq!(fields.get_current_step(), AddRecordStep::Name);
    }

    #[test]
    fn test_empty_login_still_counts_as_present() {
        let mut fields = RecordFields::new();
        fields.password = "secret".to_string();
        fields.name = "mail".to_string();
        fields.login = Some(String::new());
        assert_eq!(fields.get_current_step(), AddRecordStep::Description);
    }

    #[test]
    fn test_empty_description_still_counts_as_complete() {
        let mut fields = RecordFields::new();
        fields.password = "secret".to_string();
        fields.name = "mail".to_string();
        fields.login = Some("user".to_string());
        fields.description = Some(String::new());
        assert_eq!(fields.get_current_step(), AddRecordStep::Complete);
    }
}
