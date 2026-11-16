fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&puzzgen::openapi::document()).unwrap()
    );
}
