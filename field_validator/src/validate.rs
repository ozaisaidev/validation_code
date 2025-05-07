use serde::de::DeserializeOwned;
use serde_json::{self, Value, json};
use std::fmt;

pub trait ValidateFields {
    /// The list of required field names for this type.
    fn required_fields() -> &'static [&'static str];
}

/// Error returned when some required fields are missing.
#[derive(Debug)]
pub struct MissingFieldsError {
    pub missing_fields: Vec<String>,
}

impl fmt::Display for MissingFieldsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "missing required fields: {:?}", self.missing_fields)
    }
}

impl std::error::Error for MissingFieldsError {}

/// Check that a JSON object has all of T::required_fields() present and non-null.
pub fn validate_json_for_type<T: ValidateFields>(json: &str) -> Result<(), MissingFieldsError> {
    let v: Value = serde_json::from_str(json)
        .map_err(|_| MissingFieldsError { missing_fields: vec![] })?;
    let obj = match v.as_object() {
        Some(obj) => obj,
        None => {
            return Err(MissingFieldsError { 
                missing_fields: T::required_fields().iter().map(|s| s.to_string()).collect()
            })
        }
    };

    let mut missing = Vec::new();
    for &field in T::required_fields() {
        if !obj.contains_key(field) || obj[field].is_null() {
            missing.push(field.to_string());
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(MissingFieldsError { missing_fields: missing })
    }
}

/// Validate and then deserialize in one shot.
pub fn validate_and_deserialize<T>(json: &str) 
    -> Result<T, Box<dyn std::error::Error>>
where
    T: ValidateFields + DeserializeOwned
{
    // First check required fields:
    validate_json_for_type::<T>(json)?;
    // If ok, then deserialize to T:
    let data = serde_json::from_str(json)?;
    Ok(data)
}
pub fn handle_json_request<T>(json: &str) -> Value
where
    T: ValidateFields + DeserializeOwned,
{
    match validate_and_deserialize::<T>(json) {
        Ok(user_data) => json!({
            "statusCode": 200,
            "body": {
                "message": "Success",
                
            }
        }),

        Err(err) => {
            // if it’s our MissingFieldsError, return a 400 + missingFields
            if let Some(mf) = err.downcast_ref::<MissingFieldsError>() {
                json!({
                    "statusCode": 400,
                    "body": {
                        "error": "Validation Error",
                        "message": format!("{}", mf),
                        "missingFields": mf.missing_fields
                    }
                })
            } else {
                // any other deserialization error
                json!({
                    "statusCode": 400,
                    "body": {
                        "error": "Bad Request",
                        "message": format!("{}", err)
                    }
                })
            }
        }
    }
}