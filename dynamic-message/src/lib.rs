/* Licensed under the AGPL-3.0 License: https://www.gnu.org/licenses/agpl-3.0.html */

use prost_reflect::{
    DynamicMessage, Kind, MessageDescriptor, SerializeOptions, Value as ProstReflectValue,
};
use regex::Regex;
use serde_json::Value as JsonValue;
use tracing::{debug, error, info};

#[tracing::instrument]
pub fn populate_dynamic_message(
    dynamic_message: &mut DynamicMessage,
    message_descriptor: &MessageDescriptor,
    json_value: &JsonValue,
) -> Result<(), String> {
    info!("populate_dynamic_message");

    if let JsonValue::Object(map) = json_value {
        for (field_name, field_value) in map {
            if let Some(field_descriptor) = message_descriptor.get_field_by_name(field_name) {
                match field_descriptor.kind() {
                    Kind::Double | Kind::Float => {
                        if let Some(float_value) = field_value.as_f64() {
                            let value = ProstReflectValue::F64(float_value);
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!(
                                    "Field '{}' set to F64 with value {}",
                                    field_name, float_value
                                );
                            } else {
                                return Err(format!(
                                    "Field '{}' expects a float or double value",
                                    field_name
                                ));
                            }
                        } else {
                            return Err(format!(
                                "Field '{}' expects a float or double value",
                                field_name
                            ));
                        }
                    }
                    Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => {
                        if let Some(int_value) = field_value.as_i64() {
                            let value = ProstReflectValue::I32(int_value as i32);
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!(
                                    "Field '{}' set to I32 with value {}",
                                    field_name, int_value
                                );
                            } else {
                                return Err(format!(
                                    "Field '{}' expects an integer value",
                                    field_name
                                ));
                            }
                        } else {
                            return Err(format!("Field '{}' expects an integer value", field_name));
                        }
                    }
                    Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => {
                        if let Some(int_value) = field_value.as_i64() {
                            let value = ProstReflectValue::I64(int_value);
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!(
                                    "Field '{}' set to I64 with value {}",
                                    field_name, int_value
                                );
                            } else {
                                return Err(format!(
                                    "Field '{}' expects a 64-bit integer value",
                                    field_name
                                ));
                            }
                        } else {
                            return Err(format!(
                                "Field '{}' expects a 64-bit integer value",
                                field_name
                            ));
                        }
                    }
                    Kind::Uint32 | Kind::Fixed32 => {
                        if let Some(int_value) = field_value.as_u64() {
                            let value = ProstReflectValue::U32(int_value as u32);
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!(
                                    "Field '{}' set to U32 with value {}",
                                    field_name, int_value
                                );
                            } else {
                                return Err(format!(
                                    "Field '{}' expects an unsigned integer value",
                                    field_name
                                ));
                            }
                        } else {
                            return Err(format!(
                                "Field '{}' expects an unsigned integer value",
                                field_name
                            ));
                        }
                    }
                    Kind::Uint64 | Kind::Fixed64 => {
                        if let Some(int_value) = field_value.as_u64() {
                            let value = ProstReflectValue::U64(int_value);
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!(
                                    "Field '{}' set to U64 with value {}",
                                    field_name, int_value
                                );
                            } else {
                                return Err(format!(
                                    "Field '{}' expects an unsigned 64-bit integer value",
                                    field_name
                                ));
                            }
                        } else {
                            return Err(format!(
                                "Field '{}' expects an unsigned 64-bit integer value",
                                field_name
                            ));
                        }
                    }
                    Kind::Bool => {
                        if let Some(bool_value) = field_value.as_bool() {
                            let value = ProstReflectValue::Bool(bool_value);
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!(
                                    "Field '{}' set to Bool with value {}",
                                    field_name, bool_value
                                );
                            } else {
                                return Err(format!(
                                    "Field '{}' expects a boolean value",
                                    field_name
                                ));
                            }
                        } else {
                            return Err(format!("Field '{}' expects a boolean value", field_name));
                        }
                    }
                    Kind::String => {
                        if let Some(string_value) = field_value.as_str() {
                            let value = ProstReflectValue::String(string_value.to_string());
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!(
                                    "Field '{}' set to String with value {}",
                                    field_name, string_value
                                );
                            } else {
                                return Err(format!(
                                    "Field '{}' expects a string value",
                                    field_name
                                ));
                            }
                        } else {
                            return Err(format!("Field '{}' expects a string value", field_name));
                        }
                    }
                    Kind::Bytes => {
                        if let Some(string_value) = field_value.as_str() {
                            let bytes = string_value.as_bytes().to_vec();
                            let bytes_for_dyn_message = bytes.clone();
                            let value = ProstReflectValue::Bytes(bytes.into());
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!(
                                    "Field '{}' set to Bytes with value {:?}",
                                    field_name, bytes_for_dyn_message
                                );
                            } else {
                                return Err(format!(
                                    "Field '{}' expects a byte array value",
                                    field_name
                                ));
                            }
                        } else {
                            return Err(format!(
                                "Field '{}' expects a byte array value",
                                field_name
                            ));
                        }
                    }
                    Kind::Enum(enum_descriptor) => {
                        if let Some(enum_value) = field_value.as_str() {
                            if let Some(enum_value) = enum_descriptor.get_value_by_name(enum_value)
                            {
                                let value = ProstReflectValue::EnumNumber(enum_value.number());
                                if value.is_valid_for_field(&field_descriptor) {
                                    dynamic_message.set_field_by_name(field_name, value);
                                    debug!(
                                        "Field '{}' set to EnumNumber with value {}",
                                        field_name,
                                        enum_value.number()
                                    );
                                } else {
                                    return Err(format!(
                                        "Field '{}' expects a valid enum value",
                                        field_name
                                    ));
                                }
                            } else {
                                return Err(format!(
                                    "Invalid enum value '{}' for field '{}'",
                                    enum_value, field_name
                                ));
                            }
                        } else {
                            return Err(format!(
                                "Field '{}' expects a valid enum value as a string",
                                field_name
                            ));
                        }
                    }
                    Kind::Message(sub_message_descriptor) => {
                        if let Some(nested_value) = field_value.as_object() {
                            let mut nested_message =
                                DynamicMessage::new(sub_message_descriptor.clone());
                            populate_dynamic_message(
                                &mut nested_message,
                                &sub_message_descriptor,
                                field_value,
                            )?;
                            let value = ProstReflectValue::Message(nested_message);
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!("Field '{}' set to nested message", field_name);
                            } else {
                                return Err(format!(
                                    "Field '{}' expects a nested message object",
                                    field_name
                                ));
                            }
                        } else {
                            return Err(format!(
                                "Field '{}' expects a nested message object",
                                field_name
                            ));
                        }
                    }
                }
            } else {
                return Err(format!("Field '{}' not found in descriptor", field_name));
            }
        }
    } else {
        return Err("Expected a JSON object to populate DynamicMessage".to_string());
    }

    Ok(())
}

#[tracing::instrument]
pub fn serialize_dynamic_message(dynamic_message: &mut DynamicMessage) -> Result<Vec<u8>, String> {
    info!("serialize_dynamic_message");

    let options = SerializeOptions::new().skip_default_fields(false);

    let mut serializer = serde_json::Serializer::new(vec![]);
    dynamic_message
        .serialize_with_options(&mut serializer, &options)
        .map_err(|e| {
            error!("Failed to serialize DynamicMessage back to JSON: {:?}", e);
            format!("Failed to serialize DynamicMessage back to JSON: {:?}", e)
        })?;

    let serialized_json = String::from_utf8(serializer.into_inner()).map_err(|e| {
        error!("Failed to convert serialized data to UTF-8: {:?}", e);
        format!("Failed to convert serialized data to UTF-8: {:?}", e)
    })?;

    debug!("Serialized JSON: {:?}", serialized_json);

    Ok(serialized_json.into_bytes())
}

pub const SHORT_STRING: &str = "Hello world";
pub const LONG_STRING: &str = "This is a much longer string that contains more words";

pub fn string_split_whitespace(file: &str) -> Vec<&str> {
    file.split_whitespace().collect()
}

pub fn string_split_whitespace_regex(file: &str) -> Vec<&str> {
    Regex::new(r"\s+").unwrap().split(&file[1..]).collect()
}
