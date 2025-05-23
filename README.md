# Field Validator Crates

This document describes the `field_validator` and `field_validator_derive` crates, which provide a mechanism for validating required fields in JSON payloads before deserialization in Rust applications.

## Overview

The `field_validator` crate defines a core trait, `ValidateFields`, and associated functions to check for the presence and non-nullity of specified fields within a JSON string. It helps ensure that incoming JSON data meets basic structural requirements before attempting full deserialization. The `field_validator_derive` crate provides a procedural macro, `#[derive(ValidateFields)]`, which automatically implements the `ValidateFields` trait for your structs. It intelligently determines required fields by considering field types (non-`Option<T>` types are generally required) and specific `serde` attributes (like `default` or `skip_serializing_if`) or a custom `#[field_validator(optional)]` attribute, which mark fields as optional.

## Validation Mechanism

Validation is performed by first obtaining a list of "required" field names for a given struct. This list is provided by the `required_fields()` method of the `ValidateFields` trait, which can be implemented manually or, more commonly, automatically generated by the `#[derive(ValidateFields)]` macro. The `validate_json_for_type::<YourStructType>(&json_string)` function then parses the input JSON string into a generic JSON `Value`. It iterates through the required field names, checking if each key exists in the parsed JSON object and if its corresponding value is not `null`. If any required fields are absent or `null`, the function returns a `MissingFieldsError` detailing which fields are missing. This check occurs before the more expensive and potentially error-prone full deserialization into the target struct type.

## How to Use

Here's how to integrate and use the field validation mechanism in your project, referencing the usage in `fetch_ride_mode/src/main.rs`:

1.  **Add Dependencies:**
    Include `field_validator` and `field_validator_derive` in your `Cargo.toml`:
    ```toml
    [dependencies]
    serde = { version = "1.0", features = ["derive"] }
    serde_json = "1.0"
    field_validator = { path = "../field_validator" } # Or version from crates.io
    field_validator_derive = { path = "../field_validator_derive" } # Or version from crates.io
    ```

2.  **Import Necessary Items:**
    In your Rust file (e.g., `main.rs` or `lib.rs`):
    ```rust
    use field_validator::validate_json_for_type;
    use field_validator::ValidateFields; // The trait
    use field_validator_derive::ValidateFields; // The derive macro
    use serde::Deserialize;
    use serde_json::json; // For constructing error responses
    ```

3.  **Define Your Struct:**
    Derive `ValidateFields` and `Deserialize` for the struct you want to validate. Fields are considered required by default unless they are of type `Option<T>`, or have attributes like `#[serde(default)]`, `#[serde(skip_serializing_if = "Option::is_none")]`, or `#[field_validator(optional)]`.

    ```rust
    #[derive(ValidateFields, Deserialize)]
    struct Request {
        #[serde(rename = "bike_identifier")]
        bike_identifier: String, // Required

        #[serde(rename = "change_to_mode")]
        change_to_mode: String, // Required

        #[serde(rename = "current_mode")]
        current_mode: Option<String>, // Optional because it's Option<T>
    }
    ```

4.  **Perform Validation:**
    Before attempting to deserialize the JSON string with `serde_json::from_str`, call `validate_json_for_type`.

    ```rust
    async fn lambda_handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
        let json_str = event.payload.to_string();

        // 1) Missing-fields check
        if let Err(missing) = validate_json_for_type::<Request>(&json_str) {
            return Ok(json!({
                "statusCode": 400,
                "body": {
                    "error": "Validation Error",
                    "message": format!("{}", missing),
                    "missingFields": missing.missing_fields
                }
            }));
        }

        // 2) JSON syntax / type errors (full deserialization)
        let payload: Request = match serde_json::from_str(&json_str) {
            Ok(p) => p,
            Err(e) => {
                return Ok(json!({
                    "statusCode": 400,
                    "body": {
                        "error": "Bad Request",
                        "message": e.to_string()
                    }
                }));
            }
        };

        // ... rest of your logic using `payload`
        Ok(json!({"status": "success"}))
    }
    ```
    This approach allows you to return a specific error message indicating exactly which required fields are missing, improving the API's usability. If `validate_json_for_type` returns `Ok(())`, you can then proceed with `serde_json::from_str`, which might still fail due to type mismatches or syntax errors, but not due to missing required fields.

The `field_validator` crate also offers `validate_and_deserialize` for a combined step, and `handle_json_request` as a higher-level utility to directly produce a `serde_json::Value` response suitable for AWS Lambda or similar environments.
