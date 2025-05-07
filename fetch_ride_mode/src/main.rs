// Example code for the usage of the ValidateFields trait and validate_json_for_type function
// comments edited manually not ai
use aws_config::meta::region::RegionProviderChain;
use aws_config::load_from_env;
use aws_sdk_sns::Client as SnsClient;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use field_validator::validate_json_for_type;
use field_validator::ValidateFields;
use field_validator_derive::ValidateFields;




mod rdbc;
use crate::rdbc::get_vcu_data;

#[derive(ValidateFields,Deserialize)]
struct Request {
    #[serde(rename = "bike_identifier")]
    bike_identifier: String,
    
    #[serde(rename = "change_to_mode")]
    change_to_mode: String,
    
    #[serde(rename = "current_mode")]
    current_mode: Option<String>,
}

#[derive(Serialize)]
struct SnsPayload {
    bike_identifier: String,
    steps: usize,
}

const MODES: [&str; 3] = ["glide", "combat", "ballistic"];

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    lambda_runtime::run(service_fn(lambda_handler)).await?;
    Ok(())
}
async fn lambda_handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let json_str = event.payload.to_string();

    // 1) Missing‐fields check
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

    // 2) JSON syntax / type errors
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
//rest of the code logic 


    // value level error handling implmeneted 
    let current_index = match MODES.iter().position(|&m| m == current_mode) {
        Some(idx) => idx,
        None => {
            return Ok(json!({
                "statusCode": 400,
                "body": {
                    "error": "Invalid Field",
                    "message": format!("Invalid current_mode: `{}`", current_mode),
                    "invalidFields": ["current_mode"]
                }
            }))
        }
    };

    let target_index = match MODES.iter().position(|&m| m == target_mode) {
        Some(idx) => idx,
        None => {
            return Ok(json!({
                "statusCode": 400,
                "body": {
                    "error": "Validation Error",
                    "message": format!("Invalid change_to_mode: `{}`", target_mode),
                    "invalidFields": ["change_to_mode"]
                }
            }))
        }
    };

   //rest of the code 
}


