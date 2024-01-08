use std::collections::HashMap;

use sec_store::record::Record;

use stated_dialogues::stated_dialogues::{MessageFormat, OutgoingMessage};

pub const RECORD_NAME_FIELD: &str = "RECORD_NAME";
pub const RECORD_PASSWD_FIELD: &str = "RECORD_PASSWD";
pub const RECORD_DESCR_FIELD: &str = "RECORD_DESCR";
pub const RECORD_LOGIN_FIELD: &str = "RECORD_LOGIN";

pub fn record_as_message(record: &Record) -> OutgoingMessage {
    let fields: HashMap<String, String> = HashMap::from_iter(
        record
            .get_fields()
            .into_iter()
            .map(|(name, value)| (name.clone(), value.clone())),
    );

    let mut lines: Vec<String> = vec![format!(
        "Название: <code>{}</code>",
        fields[RECORD_NAME_FIELD]
    )];
    if let Some(login) = fields.get(RECORD_LOGIN_FIELD) {
        lines.push(format!("Логин: <code>{}</code>", login));
    }
    if let Some(descr) = fields.get(RECORD_DESCR_FIELD) {
        lines.push(format!("Описание: <code>{}</code>", descr));
    }

    lines.push(format!(
        "Пароль: <code>{}</code>",
        fields[RECORD_PASSWD_FIELD]
    ));

    OutgoingMessage::new(lines.join("\n"), MessageFormat::Html)
}
