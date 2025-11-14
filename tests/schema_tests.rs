use agentflow::{Schema, SchemaKind, SchemaRegistry, register_schema, validate_schema};
use serde_json::json;

#[test]
fn schema_registry_registers_and_validates() {
    let mut registry = SchemaRegistry::new();
    registry.register(
        "user",
        Schema::new(SchemaKind::Object {
            properties: vec![
                ("name".to_string(), Schema::new(SchemaKind::String)),
                ("age".to_string(), Schema::new(SchemaKind::Integer)),
            ]
            .into_iter()
            .collect(),
            required: vec!["name".to_string()],
            additional: false,
        }),
    );

    let value = json!({
        "name": "Alice",
        "age": 30
    });
    assert!(registry.validate("user", &value).is_ok());

    let missing = json!({ "age": 30 });
    assert!(registry.validate("user", &missing).is_err());

    let extra = json!({
        "name": "Bob",
        "city": "Paris"
    });
    assert!(registry.validate("user", &extra).is_err());
}

#[test]
fn global_registry_helpers_work() {
    register_schema(
        "simple.string",
        Schema::new(SchemaKind::String).with_name("simple.string"),
    );
    assert!(validate_schema("simple.string", &json!("ok")).is_ok());
    assert!(validate_schema("simple.string", &json!(123)).is_err());
}
