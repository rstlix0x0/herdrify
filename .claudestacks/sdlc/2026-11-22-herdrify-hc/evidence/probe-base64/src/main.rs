//! Does base64 encode/decode the shapes hc uses (standard alphabet, padded)?
use base64::{Engine, engine::general_purpose::STANDARD};
fn main() {
    println!("encode(\"hello\") = {}", STANDARD.encode(b"hello"));
    println!("encode(\"x\\n\") = {}", STANDARD.encode(b"x\n"));
    println!("decode(\"eAo=\") = {:?}", STANDARD.decode("eAo="));
    println!("decode(\"abcd\") = {:?}", STANDARD.decode("abcd"));
    println!("decode(\"not base64!\") = {:?}", STANDARD.decode("not base64!").map_err(|e| e.to_string()));
    println!("decode(\"\") = {:?}", STANDARD.decode(""));
}
