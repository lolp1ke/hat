use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};

pub fn sanitize(input: &str) -> String {
  BASE64_URL_SAFE_NO_PAD.encode(input)
}
pub fn decode_sanitized(input: &str) -> String {
  let bytes = BASE64_URL_SAFE_NO_PAD.decode(input).unwrap();
  String::from_utf8(bytes).unwrap()
}
