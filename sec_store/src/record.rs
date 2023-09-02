use serde::{Deserialize, Serialize};

use uuid::Uuid;

use crate::cipher::{decrypt_string, encrypt_string, DecryptResult, EncryptedData, EncryptionKey};

pub type RecordId = String;

pub type FieldName = String;
pub type FieldValue = String;
pub type RecordField = (FieldName, FieldValue);

pub type UpdateFieldResult<T> = anyhow::Result<T, FieldDoesntExist>;
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDoesntExist;

pub type AddFieldResult<T> = anyhow::Result<T, SameFieldAlreadyExist>;
#[derive(Debug, Clone, PartialEq)]
pub struct SameFieldAlreadyExist;

pub type EncryptedRecord = String;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Record {
    pub id: RecordId,
    fields: Vec<RecordField>,
}

impl Record {
    pub fn new(fields: Vec<RecordField>) -> Record {
        Record {
            id: Uuid::new_v4().to_string(),
            fields: fields,
        }
    }

    pub fn add_field(
        &mut self,
        field_name: FieldName,
        field_value: FieldValue,
    ) -> AddFieldResult<()> {
        match self.fields.iter().find(|(name, _)| field_name.eq(name)) {
            Some(_) => Err(SameFieldAlreadyExist),
            None => {
                self.fields.push((field_name, field_value));
                Ok(())
            }
        }
    }

    pub fn get_fields(&self) -> Vec<(&FieldName, &FieldValue)> {
        self.fields.iter().map(|(a, b)| (a, b)).collect()
    }

    pub fn update_field(
        &mut self,
        field_name: FieldName,
        field_value: FieldValue,
    ) -> UpdateFieldResult<()> {
        match self
            .fields
            .iter()
            .enumerate()
            .find(|(_, (name, _))| field_name.eq(name))
            .map(|(pos, _)| pos)
        {
            Some(idx) => {
                self.fields[idx] = (field_name, field_value);
                Ok(())
            }
            None => Err(FieldDoesntExist),
        }
    }

    pub fn encrypt(&self, passwd: EncryptionKey) -> EncryptedRecord {
        serde_json::to_string(&encrypt_string(
            passwd,
            serde_json::to_string(self).unwrap(),
        ))
        .unwrap()
    }

    pub fn decrypt(
        passwd: EncryptionKey,
        encrypted_record: &EncryptedRecord,
    ) -> DecryptResult<Record> {
        Ok(serde_json::from_str::<Record>(&decrypt_string(
            passwd,
            serde_json::from_str::<EncryptedData>(encrypted_record).unwrap(),
        )?)
        .unwrap())
    }
}

#[cfg(test)]
mod tests {
    use crate::record::Record;

    #[test]
    fn test_record_encryption() {
        let fields = vec![(String::from("First"), String::from("1"))];
        super::Record::new(fields).encrypt("password");
    }

    #[test]
    fn test_record_decryption() {
        let passwd = "password";

        let fields = vec![(String::from("First"), String::from("1"))];
        let original_record = super::Record::new(fields.clone());
        let decrypted_record =
            super::Record::decrypt(passwd, &original_record.encrypt(passwd)).unwrap();

        assert_eq!(original_record, decrypted_record);
        assert_eq!(decrypted_record.fields, fields);
    }

    #[test]
    fn test_record_decryption_with_bad_passwd() {
        let fields = vec![(String::from("First"), String::from("1"))];
        let result =
            super::Record::decrypt("Second", &super::Record::new(fields.clone()).encrypt("One"));

        let expected = crate::cipher::DecryptionError::WrongPassword;
        assert_eq!(result, Err(expected));
    }

    #[test]
    fn test_field_add() {
        let fields = vec![("Field1".to_string(), "Value1".to_string())];
        let mut record = Record::new(fields);

        record
            .add_field("Field2".to_string(), "Value2".to_string())
            .unwrap();

        assert_eq!(
            record.get_fields(),
            vec![
                (&"Field1".to_string(), &"Value1".to_string()),
                (&"Field2".to_string(), &"Value2".to_string()),
            ]
        )
    }

    #[test]
    fn test_field_add_err() {
        let fields = vec![("Field1".to_string(), "Value1".to_string())];
        let mut record = Record::new(fields);

        let result = record.add_field("Field1".to_string(), "Value2".to_string());
        assert_eq!(result, Err(crate::record::SameFieldAlreadyExist));
    }

    #[test]
    fn test_field_update() {
        let fields = vec![("Field1".to_string(), "Value1".to_string())];
        let mut record = Record::new(fields);

        record
            .update_field("Field1".to_string(), "Value2".to_string())
            .unwrap();

        assert_eq!(
            record.get_fields(),
            vec![
                (&"Field1".to_string(), &"Value2".to_string()),
            ]
        )
    }

    #[test]
    fn test_field_update_err() {
        let fields = vec![("Field1".to_string(), "Value1".to_string())];
        let mut record = Record::new(fields);

        let result = record
            .update_field("Field2".to_string(), "Value2".to_string());

        assert_eq!(result, Err(crate::record::FieldDoesntExist));
    }

}
