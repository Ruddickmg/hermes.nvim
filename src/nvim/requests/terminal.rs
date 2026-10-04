use nvim_oxi::conversion::FromObject;

use crate::acp::Result;
use crate::acp::error::Error;
use crate::nvim::configuration::dict_from_object;
use crate::nvim::terminal::parse_exit_code;

use super::request::Request;

impl Request {
    pub(super) fn parse_terminal_output_response(data: nvim_oxi::Object) -> Result<(String, bool)> {
        // First, try to parse as a plain String
        match String::from_object(data.clone()) {
            Ok(output) => Ok((output, false)),
            Err(_) => {
                // Not a string, try Dictionary
                let dict =
                    dict_from_object(data).map_err(|e| Error::InvalidInput(e.to_string()))?;

                // "output" field is required and must be a String
                let output = dict
                    .get("output")
                    .cloned()
                    .ok_or(Error::InvalidInput(
                        "Missing 'output' field in terminal output response".to_string(),
                    ))
                    .and_then(|o| {
                        String::from_object(o).map_err(|e| Error::InvalidInput(e.to_string()))
                    })?;

                // "truncated" field is optional, defaults to false
                let truncated = match dict.get("truncated").cloned() {
                    Some(t) => {
                        bool::from_object(t).map_err(|e| Error::InvalidInput(e.to_string()))?
                    }
                    None => false,
                };

                Ok((output, truncated))
            }
        }
    }

    pub(super) fn parse_terminal_exit_response(
        data: nvim_oxi::Object,
    ) -> Result<(Option<u32>, Option<String>)> {
        // First, try to parse as a plain String (signal name only)
        match String::from_object(data.clone()) {
            Ok(signal) => Ok(if signal.is_empty() {
                return Err(Error::InvalidInput(
                    "Signal string cannot be empty".to_string(),
                ));
            } else {
                (None, Some(signal))
            }),
            Err(_) => {
                // Not a string, try Integer (exit code)
                match i64::from_object(data.clone()) {
                    Ok(exit_code) => Ok(parse_exit_code(exit_code)),
                    Err(_) => {
                        let dict = dict_from_object(data)
                            .map_err(|e| Error::InvalidInput(e.to_string()))?;

                        // "exitCode" field is optional
                        let exit_code = match dict.get("exitCode").cloned() {
                            Some(ec) => {
                                let code: i64 = i64::from_object(ec)
                                    .map_err(|e| Error::InvalidInput(e.to_string()))?;
                                Some(code)
                            }
                            None => None,
                        };

                        // "signal" field is optional
                        let signal = match dict.get("signal").cloned() {
                            Some(s) => {
                                let sig: String = String::from_object(s)
                                    .map_err(|e| Error::InvalidInput(e.to_string()))?;
                                if sig.is_empty() { None } else { Some(sig) }
                            }
                            None => None,
                        };

                        if signal.is_none() && exit_code.is_none() {
                            Err(Error::InvalidInput(
                                "Terminal exit response must contain at least 'exitCode' or 'signal'".to_string(),
                            ))
                        } else if let Some(code) = exit_code {
                            let (parsed_exit_code, parsed_signal) = parse_exit_code(code);
                            let final_signal = match (signal, parsed_signal) {
                                (Some(explicit_sig), _) => Some(explicit_sig),
                                (None, inferred_sig) => inferred_sig,
                            };
                            Ok((parsed_exit_code, final_signal))
                        } else {
                            Ok((None, signal))
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvim_oxi::Object;

    #[test]
    fn parse_terminal_output_accepts_plain_string() {
        let obj = Object::from("hello world");
        let result = Request::parse_terminal_output_response(obj);
        assert_eq!(result.unwrap(), ("hello world".to_string(), false));
    }

    #[test]
    fn parse_terminal_output_accepts_empty_string() {
        let obj = Object::from("");
        let result = Request::parse_terminal_output_response(obj);
        assert_eq!(result.unwrap(), ("".to_string(), false));
    }

    #[test]
    fn parse_terminal_output_accepts_dictionary_with_output_field() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("output", Object::from("test output"));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_output_response(obj);
        assert_eq!(result.unwrap(), ("test output".to_string(), false));
    }

    #[test]
    fn parse_terminal_output_accepts_dictionary_with_truncated_true() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("output", Object::from("test output"));
        dict.insert("truncated", Object::from(true));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_output_response(obj);
        assert_eq!(result.unwrap(), ("test output".to_string(), true));
    }

    #[test]
    fn parse_terminal_output_accepts_dictionary_with_truncated_false() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("output", Object::from("test output"));
        dict.insert("truncated", Object::from(false));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_output_response(obj);
        assert_eq!(result.unwrap(), ("test output".to_string(), false));
    }

    #[test]
    fn parse_terminal_output_rejects_missing_output_field() {
        let dict = nvim_oxi::Dictionary::default();
        let obj = Object::from(dict);
        let result = Request::parse_terminal_output_response(obj);
        assert!(result.is_err());
    }

    #[test]
    fn parse_terminal_output_rejects_invalid_output_type() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("output", Object::from(123i64));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_output_response(obj);
        assert!(result.is_err());
    }

    #[test]
    fn parse_terminal_output_rejects_invalid_truncated_type() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("output", Object::from("test"));
        dict.insert("truncated", Object::from("yes"));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_output_response(obj);
        assert!(result.is_err());
    }

    #[test]
    fn parse_terminal_exit_response_accepts_signal_string() {
        let obj = Object::from("SIGTERM");
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (None, Some("SIGTERM".to_string())));
    }

    #[test]
    fn parse_terminal_exit_response_accepts_exit_code_integer() {
        let obj = Object::from(42i64);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (Some(42), None));
    }

    #[test]
    fn parse_terminal_exit_response_accepts_exit_code_zero() {
        let obj = Object::from(0i64);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (Some(0), None));
    }

    #[test]
    fn parse_terminal_exit_response_accepts_negative_signal_number() {
        let obj = Object::from(-9i64);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (None, Some("SIGKILL".to_string())));
    }

    #[test]
    fn parse_terminal_exit_response_accepts_exit_code_128_plus_range() {
        // 137 = 128 + 9, should return BOTH exit code AND signal
        let obj = Object::from(137i64);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (Some(137), Some("SIGKILL".to_string())));
    }

    #[test]
    fn parse_terminal_exit_response_accepts_dictionary_with_both_fields() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("exitCode", Object::from(9i64));
        dict.insert("signal", Object::from("SIGKILL"));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        // Exit code 9 is in 0-127 range, so parse_exit_code returns (Some(9), None)
        // But explicit signal from dict takes precedence over inferred signal
        assert_eq!(result.unwrap(), (Some(9), Some("SIGKILL".to_string())));
    }

    #[test]
    fn parse_terminal_exit_response_accepts_dictionary_exit_code_only() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("exitCode", Object::from(42i64));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (Some(42), None));
    }

    #[test]
    fn parse_terminal_exit_response_accepts_dictionary_signal_only() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("signal", Object::from("SIGTERM"));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (None, Some("SIGTERM".to_string())));
    }

    #[test]
    fn parse_terminal_exit_response_handles_empty_signal_string() {
        let obj = Object::from("");
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_err());
    }

    #[test]
    fn parse_terminal_exit_response_handles_empty_dictionary_signal() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("exitCode", Object::from(1i64));
        dict.insert("signal", Object::from(""));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        // When signal is empty but exitCode is present, signal becomes None
        assert_eq!(result.unwrap(), (Some(1), None));
    }

    #[test]
    fn parse_terminal_exit_response_handles_unknown_negative_signal() {
        let obj = Object::from(-999i64);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (None, Some("UNKNOWN(-999)".to_string())));
    }

    #[test]
    fn parse_terminal_exit_response_handles_unknown_128_plus_range() {
        // 255 = 128 + 127, which is in the 128..=255 range
        // 127 is not a standard signal, so map_codes returns None
        let obj = Object::from(255i64);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (Some(255), None));
    }
}
