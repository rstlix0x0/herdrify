fn main() {
    let shortest = r#"{"method":"event","params":{"seq":9,"type":"x"}}"#;
    let stale = r#"{"method":"event","params":{"seq":9,"type":"removed"}}"#;
    let fresh = r#"{"method":"event","params":{"seq":11,"type":"title","data":{"title":"abc"}}}"#;
    println!("shortest decodable event line: {} bytes", shortest.len());
    println!("stale line: {} bytes", stale.len());
    println!("fresh line: {} bytes", fresh.len());
    println!("frame = stale + \\n + fresh + \\n + 1 byte = {} bytes", stale.len() + 1 + fresh.len() + 1 + 1);
}
