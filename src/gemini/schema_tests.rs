// Tool schemas: lightly cleaned for parametersJsonSchema, whitelisted for `parameters`.

use super::*;

#[test]
fn clean_drops_meta_keys_but_not_properties_named_like_them() {
    let s = json!({
        "$schema": "http://json-schema.org/draft-07/schema#", "$id": "x", "$comment": "c",
        "type": "object",
        "properties": {
            "examples": { "type": "string", "examples": ["a"] },
            "mode": { "const": "fast" },
            "list": { "type": "array", "items": { "$comment": "c", "type": "string" } }
        },
        "required": ["examples"],
        "additionalProperties": false
    });
    assert_eq!(clean(&s), json!({
        "type": "object",
        "properties": {
            "examples": { "type": "string" },
            "mode": { "enum": ["fast"] },
            "list": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["examples"],
        "additionalProperties": false
    }));
}

#[test]
fn legacy_keeps_only_known_keys() {
    let s = json!({
        "$schema": "x", "type": "object", "title": "T", "additionalProperties": false,
        "properties": {
            "n": { "type": "integer", "minimum": 1, "exclusiveMaximum": 9, "default": 2 },
            "s": { "type": "string", "format": "uri", "minLength": 1, "pattern": "^a" },
            "d": { "type": "string", "format": "date-time" }
        },
        "required": ["n", "ghost"]
    });
    assert_eq!(legacy(&s), json!({
        "type": "object",
        "properties": {
            "n": { "type": "integer", "minimum": 1 },
            "s": { "type": "string", "minLength": 1, "pattern": "^a" },
            "d": { "type": "string", "format": "date-time" }
        },
        "required": ["n"]
    }));
}

#[test]
fn legacy_turns_a_null_type_into_nullable() {
    let s = json!({ "type": "object", "properties": {
        "a": { "type": ["string", "null"] },
        "b": { "anyOf": [{ "type": "number" }, { "type": "null" }], "description": "B" }
    }});
    assert_eq!(legacy(&s)["properties"], json!({
        "a": { "type": "string", "nullable": true },
        "b": { "type": "number", "nullable": true, "description": "B" }
    }));
}

#[test]
fn legacy_turns_one_of_into_any_of_and_keeps_enums_on_strings_only() {
    let s = json!({ "type": "object", "properties": {
        "a": { "oneOf": [{ "type": "string" }, { "type": "integer" }] },
        "b": { "type": "integer", "enum": [1, 2] },
        "c": { "type": "string", "enum": ["x", "y"] },
        "d": { "const": "only" }
    }});
    assert_eq!(legacy(&s)["properties"], json!({
        "a": { "anyOf": [{ "type": "string" }, { "type": "integer" }] },
        "b": { "type": "integer" },
        "c": { "type": "string", "enum": ["x", "y"] },
        "d": { "type": "string", "enum": ["only"] }
    }));
}

#[test]
fn legacy_inlines_refs_and_stops_at_a_loop() {
    let s = json!({
        "type": "object",
        "properties": {
            "p": { "$ref": "#/$defs/Point" },
            "tree": { "$ref": "#/definitions/Node" }
        },
        "$defs": { "Point": { "type": "object", "properties": { "x": { "type": "number" } } } },
        "definitions": { "Node": { "type": "object", "properties": { "kids": { "type": "array", "items": { "$ref": "#/definitions/Node" } } } } }
    });
    let out = legacy(&s);
    assert_eq!(out["properties"]["p"], json!({ "type": "object", "properties": { "x": { "type": "number" } } }));
    assert_eq!(out["properties"]["tree"]["properties"]["kids"]["items"], json!({ "type": "object" }));
    assert!(out.get("$defs").is_none() && out.get("definitions").is_none());
}

#[test]
fn legacy_merges_all_of_and_fills_what_gemini_needs() {
    let s = json!({ "properties": {
        "both": { "allOf": [
            { "type": "object", "properties": { "a": { "type": "string" } }, "required": ["a"] },
            { "properties": { "b": { "type": "boolean" } } }
        ]},
        "arr": { "type": "array" },
        "empty": { "type": "object", "properties": {} }
    }});
    assert_eq!(legacy(&s), json!({ "type": "object", "properties": {
        "both": { "type": "object", "properties": { "a": { "type": "string" }, "b": { "type": "boolean" } }, "required": ["a"] },
        "arr": { "type": "array", "items": { "type": "object" } },
        "empty": { "type": "object" }
    }}));
}
