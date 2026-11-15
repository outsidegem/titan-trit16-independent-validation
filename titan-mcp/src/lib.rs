//! Titan Trit16 SDK — Local-First Model Context Protocol (MCP) JSON-RPC 2.0 Server
//!
//! Exposes `titan_trit16_core` operations over:
//! - Standard I/O (`--stdio`, newline-delimited JSON-RPC 2.0)
//! - Unix Domain Socket (`--socket <path>`)
//! - Localhost TCP Loopback (`--tcp 127.0.0.1:<port>`)
//!
//! Enforces `#![forbid(unsafe_code)]` and a strict 64 KiB (`65,536` bytes) per-request
//! payload size limit prior to allocation/parsing.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use titan_trit16_core::{Kleene, Trit16, TritError};

/// Maximum allowed JSON-RPC request size in bytes (64 KiB).
pub const MAX_PAYLOAD_BYTES: usize = 65_536;

/// Maximum number of concurrent socket client threads allowed simultaneously.
pub const MAX_CONCURRENT_CLIENTS: usize = 64;

/// JSON-RPC 2.0 standard error codes and Titan domain error codes.
pub const ERR_PARSE_ERROR: i32 = -32700;
pub const ERR_INVALID_REQUEST: i32 = -32600;
pub const ERR_METHOD_NOT_FOUND: i32 = -32601;
pub const ERR_INVALID_PARAMS: i32 = -32602;
pub const ERR_TRAP_DETECTED: i32 = -32001;
pub const ERR_INVALID_INDEX: i32 = -32002;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
    #[serde(default)]
    pub id: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Value,
}

impl JsonRpcResponse {
    pub fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn err(id: Value, code: i32, message: impl Into<String>, data: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data,
            }),
            id,
        }
    }
}

fn trap_indices(mask: u16) -> Vec<u8> {
    (0..16).filter(|&i| ((mask >> i) & 1) != 0).collect()
}

fn word_to_json(w: Trit16) -> Value {
    let trits: Vec<&str> = (0..16)
        .map(|i| match w.get(i).unwrap_or(Kleene::Unknown) {
            Kleene::Unknown => "0",
            Kleene::True => "+1",
            Kleene::False => "-1",
            Kleene::Trap => "TRAP",
        })
        .collect();
    let mask = w.trap_mask();
    json!({
        "pos": w.pos(),
        "neg": w.neg(),
        "interleaved": w.to_u32_interleaved(),
        "isValid": w.is_valid(),
        "hasTraps": w.has_traps(),
        "trapMask": mask,
        "trapIndices": trap_indices(mask),
        "trits": trits,
        "display": w.to_string(),
    })
}

fn parse_u16_field(params: &Value, key: &str) -> Result<u16, JsonRpcError> {
    let val = params.get(key).ok_or_else(|| JsonRpcError {
        code: ERR_INVALID_PARAMS,
        message: format!("Missing required parameter '{key}'"),
        data: None,
    })?;
    let n = val.as_u64().ok_or_else(|| JsonRpcError {
        code: ERR_INVALID_PARAMS,
        message: format!("Parameter '{key}' must be an unsigned integer in 0..=65535"),
        data: None,
    })?;
    u16::try_from(n).map_err(|_| JsonRpcError {
        code: ERR_INVALID_PARAMS,
        message: format!("Parameter '{key}' out of u16 range (0..=65535): {n}"),
        data: None,
    })
}

fn parse_u32_field(params: &Value, key: &str) -> Result<u32, JsonRpcError> {
    let val = params.get(key).ok_or_else(|| JsonRpcError {
        code: ERR_INVALID_PARAMS,
        message: format!("Missing required parameter '{key}'"),
        data: None,
    })?;
    let n = val.as_u64().ok_or_else(|| JsonRpcError {
        code: ERR_INVALID_PARAMS,
        message: format!("Parameter '{key}' must be an unsigned 32-bit integer"),
        data: None,
    })?;
    u32::try_from(n).map_err(|_| JsonRpcError {
        code: ERR_INVALID_PARAMS,
        message: format!("Parameter '{key}' out of u32 range: {n}"),
        data: None,
    })
}

fn parse_kleene_value(v: &Value, name: &str) -> Result<Kleene, JsonRpcError> {
    if let Some(s) = v.as_str() {
        match s.trim() {
            "0" | "Unknown" | "UNKNOWN" | "unknown" => Ok(Kleene::Unknown),
            "+1" | "1" | "True" | "TRUE" | "true" => Ok(Kleene::True),
            "-1" | "False" | "FALSE" | "false" => Ok(Kleene::False),
            "TRAP" | "Trap" | "trap" | "!" => Ok(Kleene::Trap),
            other => Err(JsonRpcError {
                code: ERR_INVALID_PARAMS,
                message: format!("Invalid Kleene string for '{name}': '{other}'"),
                data: None,
            }),
        }
    } else if let Some(i) = v.as_i64() {
        match i {
            -1 => Ok(Kleene::False),
            0 => Ok(Kleene::Unknown),
            1 => Ok(Kleene::True),
            2 => Ok(Kleene::Trap),
            _ => Err(JsonRpcError {
                code: ERR_INVALID_PARAMS,
                message: format!(
                    "Invalid Kleene integer for '{name}': {i} (expected -1, 0, 1, or 2 for Trap)"
                ),
                data: None,
            }),
        }
    } else {
        Err(JsonRpcError {
            code: ERR_INVALID_PARAMS,
            message: format!("Parameter '{name}' must be a Kleene string or integer"),
            data: None,
        })
    }
}

fn parse_word_operand(obj: &Value, strict: bool) -> Result<Trit16, JsonRpcError> {
    if let Some(interleaved_val) = obj.get("interleaved") {
        let raw = interleaved_val
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| JsonRpcError {
                code: ERR_INVALID_PARAMS,
                message: "Field 'interleaved' must be a valid u32".to_string(),
                data: None,
            })?;
        if strict {
            Trit16::try_from_u32_interleaved(raw).map_err(trit_err_to_rpc)
        } else {
            Ok(Trit16::from_u32_interleaved(raw))
        }
    } else {
        let pos = parse_u16_field(obj, "pos")?;
        let neg = parse_u16_field(obj, "neg")?;
        if strict {
            Trit16::try_from_planes(pos, neg).map_err(trit_err_to_rpc)
        } else {
            Ok(Trit16::from_planes(pos, neg))
        }
    }
}

fn trit_err_to_rpc(err: TritError) -> JsonRpcError {
    match err {
        TritError::TrapDetected(mask) => JsonRpcError {
            code: ERR_TRAP_DETECTED,
            message: format!("Hardware trap detected (mask: 0x{mask:04X})"),
            data: Some(json!({
                "trapMask": mask,
                "trapIndices": trap_indices(mask),
            })),
        },
        TritError::InvalidIndex(idx) => JsonRpcError {
            code: ERR_INVALID_INDEX,
            message: format!("Trit index out of range: {idx} (must be 0..=15)"),
            data: Some(json!({ "index": idx })),
        },
    }
}

/// Executes one of the 6 Titan MCP tools directly and returns its JSON result or error.
pub fn dispatch_tool(tool_name: &str, args: &Value) -> Result<Value, JsonRpcError> {
    let strict = args.get("strict").and_then(Value::as_bool).unwrap_or(false);

    match tool_name {
        "titan_encode" => {
            if let Some(trits_val) = args.get("trits") {
                let trits_arr = trits_val.as_array().ok_or_else(|| JsonRpcError {
                    code: ERR_INVALID_PARAMS,
                    message: "Parameter 'trits' must be an array of 16 Kleene values".to_string(),
                    data: None,
                })?;
                if trits_arr.len() != 16 {
                    return Err(JsonRpcError {
                        code: ERR_INVALID_PARAMS,
                        message: format!(
                            "'trits' array must have exactly 16 elements, got {}",
                            trits_arr.len()
                        ),
                        data: None,
                    });
                }
                let mut w = Trit16::zero();
                for (i, item) in trits_arr.iter().enumerate() {
                    let k = parse_kleene_value(item, "trits[i]")?;
                    w.set(i as u8, k).map_err(trit_err_to_rpc)?;
                }
                if strict && w.has_traps() {
                    return Err(trit_err_to_rpc(TritError::TrapDetected(w.trap_mask())));
                }
                Ok(word_to_json(w))
            } else {
                let pos = parse_u16_field(args, "pos")?;
                let neg = parse_u16_field(args, "neg")?;
                let w = if strict {
                    Trit16::try_from_planes(pos, neg).map_err(trit_err_to_rpc)?
                } else {
                    Trit16::from_planes(pos, neg)
                };
                Ok(word_to_json(w))
            }
        }
        "titan_decode" => {
            let interleaved = parse_u32_field(args, "interleaved")?;
            let w = if strict {
                Trit16::try_from_u32_interleaved(interleaved).map_err(trit_err_to_rpc)?
            } else {
                Trit16::from_u32_interleaved(interleaved)
            };
            Ok(word_to_json(w))
        }
        "titan_validate" => {
            let w = parse_word_operand(args, false)?;
            let mask = w.trap_mask();
            Ok(json!({
                "isValid": w.is_valid(),
                "hasTraps": w.has_traps(),
                "trapMask": mask,
                "trapCount": mask.count_ones(),
                "trapIndices": trap_indices(mask),
            }))
        }
        "titan_detect_traps" => {
            let w = parse_word_operand(args, false)?;
            let mask = w.trap_mask();
            if strict && mask != 0 {
                return Err(trit_err_to_rpc(TritError::TrapDetected(mask)));
            }
            Ok(json!({
                "hasTraps": w.has_traps(),
                "trapMask": mask,
                "trapCount": mask.count_ones(),
                "trapIndices": trap_indices(mask),
            }))
        }
        "titan_evaluate" => {
            let op = args
                .get("op")
                .and_then(Value::as_str)
                .ok_or_else(|| JsonRpcError {
                    code: ERR_INVALID_PARAMS,
                    message: "Missing required string parameter 'op'".to_string(),
                    data: None,
                })?;

            match op {
                "and" | "or" | "collide" => {
                    let a_obj = args.get("a").ok_or_else(|| JsonRpcError {
                        code: ERR_INVALID_PARAMS,
                        message: "Missing operand 'a'".to_string(),
                        data: None,
                    })?;
                    let b_obj = args.get("b").ok_or_else(|| JsonRpcError {
                        code: ERR_INVALID_PARAMS,
                        message: "Missing operand 'b'".to_string(),
                        data: None,
                    })?;
                    let a = parse_word_operand(a_obj, false)?;
                    let b = parse_word_operand(b_obj, false)?;
                    let out = match (op, strict) {
                        ("and", false) => a.and(b),
                        ("and", true) => a.and_strict(b).map_err(trit_err_to_rpc)?,
                        ("or", false) => a.or(b),
                        ("or", true) => a.or_strict(b).map_err(trit_err_to_rpc)?,
                        ("collide", false) => a.collide(b),
                        ("collide", true) => a.collide_strict(b).map_err(trit_err_to_rpc)?,
                        _ => unreachable!(),
                    };
                    Ok(word_to_json(out))
                }
                "not" => {
                    let a_obj = args.get("a").ok_or_else(|| JsonRpcError {
                        code: ERR_INVALID_PARAMS,
                        message: "Missing operand 'a'".to_string(),
                        data: None,
                    })?;
                    let a = parse_word_operand(a_obj, false)?;
                    let out = if strict {
                        a.not_strict().map_err(trit_err_to_rpc)?
                    } else {
                        a.not()
                    };
                    Ok(word_to_json(out))
                }
                "scalar_and" | "scalar_or" => {
                    let ka = parse_kleene_value(
                        args.get("a").ok_or_else(|| JsonRpcError {
                            code: ERR_INVALID_PARAMS,
                            message: "Missing scalar operand 'a'".to_string(),
                            data: None,
                        })?,
                        "a",
                    )?;
                    let kb = parse_kleene_value(
                        args.get("b").ok_or_else(|| JsonRpcError {
                            code: ERR_INVALID_PARAMS,
                            message: "Missing scalar operand 'b'".to_string(),
                            data: None,
                        })?,
                        "b",
                    )?;
                    let res = if op == "scalar_and" {
                        Kleene::and(ka, kb)
                    } else {
                        Kleene::or(ka, kb)
                    };
                    if strict && res == Kleene::Trap {
                        return Err(trit_err_to_rpc(TritError::TrapDetected(1)));
                    }
                    Ok(json!({
                        "state": res.to_string(),
                        "isTrap": res == Kleene::Trap,
                    }))
                }
                "scalar_not" => {
                    let ka = parse_kleene_value(
                        args.get("a").ok_or_else(|| JsonRpcError {
                            code: ERR_INVALID_PARAMS,
                            message: "Missing scalar operand 'a'".to_string(),
                            data: None,
                        })?,
                        "a",
                    )?;
                    let res = Kleene::not(ka);
                    if strict && res == Kleene::Trap {
                        return Err(trit_err_to_rpc(TritError::TrapDetected(1)));
                    }
                    Ok(json!({
                        "state": res.to_string(),
                        "isTrap": res == Kleene::Trap,
                    }))
                }
                "telemetry" => {
                    let v = parse_kleene_value(
                        args.get("voltageOk").ok_or_else(|| JsonRpcError {
                            code: ERR_INVALID_PARAMS,
                            message: "Missing 'voltageOk'".to_string(),
                            data: None,
                        })?,
                        "voltageOk",
                    )?;
                    let t = parse_kleene_value(
                        args.get("thermalOk").ok_or_else(|| JsonRpcError {
                            code: ERR_INVALID_PARAMS,
                            message: "Missing 'thermalOk'".to_string(),
                            data: None,
                        })?,
                        "thermalOk",
                    )?;
                    let p = parse_kleene_value(
                        args.get("packetOk").ok_or_else(|| JsonRpcError {
                            code: ERR_INVALID_PARAMS,
                            message: "Missing 'packetOk'".to_string(),
                            data: None,
                        })?,
                        "packetOk",
                    )?;
                    let res = Kleene::evaluate_telemetry(v, t, p);
                    if strict && res == Kleene::Trap {
                        return Err(trit_err_to_rpc(TritError::TrapDetected(1)));
                    }
                    Ok(json!({
                        "health": res.to_string(),
                        "isTrap": res == Kleene::Trap,
                    }))
                }
                other => Err(JsonRpcError {
                    code: ERR_INVALID_PARAMS,
                    message: format!("Unsupported operation '{other}' (expected and, or, not, collide, scalar_and, scalar_or, scalar_not, telemetry)"),
                    data: None,
                }),
            }
        }
        "titan_collide" => {
            let a_obj = args.get("a").ok_or_else(|| JsonRpcError {
                code: ERR_INVALID_PARAMS,
                message: "Missing operand 'a'".to_string(),
                data: None,
            })?;
            let b_obj = args.get("b").ok_or_else(|| JsonRpcError {
                code: ERR_INVALID_PARAMS,
                message: "Missing operand 'b'".to_string(),
                data: None,
            })?;
            let a = parse_word_operand(a_obj, false)?;
            let b = parse_word_operand(b_obj, false)?;
            let out = if strict {
                a.collide_strict(b).map_err(trit_err_to_rpc)?
            } else {
                a.collide(b)
            };
            Ok(word_to_json(out))
        }
        other => Err(JsonRpcError {
            code: ERR_METHOD_NOT_FOUND,
            message: format!("Unknown Titan MCP tool: '{other}'"),
            data: None,
        }),
    }
}

fn mcp_tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "titan_encode",
                "description": "Encode a 16-trit word from (pos, neg) bit-planes or 16 trit states into interleaved u32 and bit-plane representations.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "pos": { "type": "integer", "minimum": 0, "maximum": 65535 },
                        "neg": { "type": "integer", "minimum": 0, "maximum": 65535 },
                        "trits": { "type": "array", "minItems": 16, "maxItems": 16 },
                        "strict": { "type": "boolean", "default": false }
                    }
                }
            },
            {
                "name": "titan_decode",
                "description": "Decode a 32-bit interleaved word into (pos, neg) planes, per-trit states, and trap diagnostics.",
                "inputSchema": {
                    "type": "object",
                    "required": ["interleaved"],
                    "properties": {
                        "interleaved": { "type": "integer", "minimum": 0, "maximum": 4294967295u64 },
                        "strict": { "type": "boolean", "default": false }
                    }
                }
            },
            {
                "name": "titan_validate",
                "description": "Check whether a 16-trit word is free of hardware Trap (11) states.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "pos": { "type": "integer", "minimum": 0, "maximum": 65535 },
                        "neg": { "type": "integer", "minimum": 0, "maximum": 65535 },
                        "interleaved": { "type": "integer", "minimum": 0, "maximum": 4294967295u64 }
                    }
                }
            },
            {
                "name": "titan_detect_traps",
                "description": "Return the 16-bit trap bitmask (pos & neg) and trap indices for a 16-trit word.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "pos": { "type": "integer", "minimum": 0, "maximum": 65535 },
                        "neg": { "type": "integer", "minimum": 0, "maximum": 65535 },
                        "interleaved": { "type": "integer", "minimum": 0, "maximum": 4294967295u64 },
                        "strict": { "type": "boolean", "default": false }
                    }
                }
            },
            {
                "name": "titan_evaluate",
                "description": "Evaluate 16-trit bit-plane operations (and, or, not, collide) or scalar Kleene/telemetry operations.",
                "inputSchema": {
                    "type": "object",
                    "required": ["op"],
                    "properties": {
                        "op": { "type": "string", "enum": ["and", "or", "not", "collide", "scalar_and", "scalar_or", "scalar_not", "telemetry"] },
                        "a": {},
                        "b": {},
                        "strict": { "type": "boolean", "default": false }
                    }
                }
            },
            {
                "name": "titan_collide",
                "description": "Perform 16-trit parallel COLLIDE consensus between operands 'a' and 'b'.",
                "inputSchema": {
                    "type": "object",
                    "required": ["a", "b"],
                    "properties": {
                        "a": { "type": "object" },
                        "b": { "type": "object" },
                        "strict": { "type": "boolean", "default": false }
                    }
                }
            }
        ]
    })
}

/// Processes a single JSON-RPC 2.0 request line and returns the serialized JSON-RPC response
/// (or `None` for JSON-RPC notifications where the `"id"` field is omitted).
pub fn handle_jsonrpc_line(line: &str) -> Option<String> {
    let line = line
        .strip_suffix("\r\n")
        .or_else(|| line.strip_suffix('\n'))
        .unwrap_or(line);
    if line.len() > MAX_PAYLOAD_BYTES {
        let resp = JsonRpcResponse::err(
            Value::Null,
            ERR_INVALID_REQUEST,
            format!(
                "Request payload length ({}) exceeds maximum allowed {} bytes",
                line.len(),
                MAX_PAYLOAD_BYTES
            ),
            None,
        );
        return serde_json::to_string(&resp).ok();
    }

    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let raw_val: Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(e) => {
            let resp = JsonRpcResponse::err(
                Value::Null,
                ERR_PARSE_ERROR,
                format!("Invalid JSON payload: {e}"),
                None,
            );
            return serde_json::to_string(&resp).ok();
        }
    };

    let Some(obj) = raw_val.as_object() else {
        let resp = JsonRpcResponse::err(
            Value::Null,
            ERR_INVALID_REQUEST,
            "JSON-RPC 2.0 request must be a JSON object",
            None,
        );
        return serde_json::to_string(&resp).ok();
    };

    let has_id = obj.contains_key("id");
    let raw_id = obj.get("id").cloned().unwrap_or(Value::Null);
    if has_id && (raw_id.is_object() || raw_id.is_array()) {
        let resp = JsonRpcResponse::err(
            Value::Null,
            ERR_INVALID_REQUEST,
            "Field 'id' must be a string, number, or null",
            None,
        );
        return serde_json::to_string(&resp).ok();
    }

    if obj.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        let resp = JsonRpcResponse::err(
            raw_id,
            ERR_INVALID_REQUEST,
            "Field 'jsonrpc' must be '2.0'",
            None,
        );
        return serde_json::to_string(&resp).ok();
    }

    let Some(method) = obj.get("method").and_then(Value::as_str) else {
        let resp = JsonRpcResponse::err(
            raw_id,
            ERR_INVALID_REQUEST,
            "Missing or non-string 'method' field",
            None,
        );
        return serde_json::to_string(&resp).ok();
    };

    let params = match obj.get("params") {
        None | Some(Value::Null) => json!({}),
        Some(v) if v.is_object() || v.is_array() => v.clone(),
        Some(_) => {
            if !has_id {
                return None;
            }
            let resp = JsonRpcResponse::err(
                raw_id,
                ERR_INVALID_REQUEST,
                "Field 'params' must be an object or array",
                None,
            );
            return serde_json::to_string(&resp).ok();
        }
    };

    // Notifications (where the `"id"` key is omitted) do not produce a response.
    // Requests with an explicit `"id": null` DO produce a response with `"id": null`.
    if !has_id {
        return None;
    }
    let id = raw_id;

    let resp = match method {
        "initialize" => JsonRpcResponse::ok(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "mcpProtocol": "1.2",
                "serverInfo": {
                    "name": "titan-mcp-server",
                    "version": env!("CARGO_PKG_VERSION"),
                    "abiVersion": "1.0.0"
                },
                "capabilities": {
                    "tools": {}
                }
            }),
        ),
        "ping" => JsonRpcResponse::ok(id, json!({})),
        "tools/list" => JsonRpcResponse::ok(id, mcp_tools_list()),
        "tools/call" => {
            let Some(tool_name) = params.get("name").and_then(Value::as_str) else {
                let err_resp = JsonRpcResponse::err(
                    id,
                    ERR_INVALID_PARAMS,
                    "Missing or non-string tool 'name' in tools/call params",
                    None,
                );
                return serde_json::to_string(&err_resp).ok();
            };
            let args = match params.get("arguments") {
                None | Some(Value::Null) => json!({}),
                Some(v) if v.is_object() => v.clone(),
                Some(_) => {
                    let err_resp = JsonRpcResponse::err(
                        id,
                        ERR_INVALID_PARAMS,
                        "Field 'arguments' in tools/call params must be a JSON object",
                        None,
                    );
                    return serde_json::to_string(&err_resp).ok();
                }
            };
            match dispatch_tool(tool_name, &args) {
                Ok(val) => JsonRpcResponse::ok(
                    id,
                    json!({
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string(&val).unwrap_or_default()
                            }
                        ],
                        "structuredContent": val,
                        "isError": false
                    }),
                ),
                Err(e) => JsonRpcResponse::err(id, e.code, e.message, e.data),
            }
        }
        direct_tool @ ("titan_encode" | "titan_decode" | "titan_validate"
        | "titan_detect_traps" | "titan_evaluate" | "titan_collide") => {
            match dispatch_tool(direct_tool, &params) {
                Ok(val) => JsonRpcResponse::ok(id, val),
                Err(e) => JsonRpcResponse::err(id, e.code, e.message, e.data),
            }
        }
        other => JsonRpcResponse::err(
            id,
            ERR_METHOD_NOT_FOUND,
            format!("Method not found: '{other}'"),
            None,
        ),
    };

    serde_json::to_string(&resp).ok()
}

/// Reads newline-delimited frames bounded to `MAX_PAYLOAD_BYTES` payload bytes
/// to prevent unbounded allocation on oversized lines without newlines.
pub fn process_stream<R: Read, W: Write>(reader: R, mut writer: W) -> std::io::Result<()> {
    let mut buf_reader = BufReader::new(reader);
    loop {
        let mut line_bytes = Vec::new();
        let bytes_read = (&mut buf_reader)
            .take((MAX_PAYLOAD_BYTES as u64) + 2)
            .read_until(b'\n', &mut line_bytes)?;
        if bytes_read == 0 {
            break;
        }
        let ended_with_newline = line_bytes.ends_with(b"\n");
        let payload_bytes = line_bytes
            .strip_suffix(b"\r\n")
            .or_else(|| line_bytes.strip_suffix(b"\n"))
            .unwrap_or(&line_bytes);

        if payload_bytes.len() > MAX_PAYLOAD_BYTES {
            if !ended_with_newline {
                // Drain remainder of oversized line up to and including '\n' without consuming subsequent lines
                loop {
                    let available = buf_reader.fill_buf()?;
                    if available.is_empty() {
                        break;
                    }
                    if let Some(pos) = available.iter().position(|&b| b == b'\n') {
                        buf_reader.consume(pos + 1);
                        break;
                    }
                    let len = available.len();
                    buf_reader.consume(len);
                }
            }
            let err_resp = JsonRpcResponse::err(
                Value::Null,
                ERR_INVALID_REQUEST,
                format!("Request payload exceeds maximum {MAX_PAYLOAD_BYTES} bytes"),
                None,
            );
            if let Ok(s) = serde_json::to_string(&err_resp) {
                writeln!(writer, "{s}")?;
                writer.flush()?;
            }
            continue;
        }

        let line_str = match std::str::from_utf8(payload_bytes) {
            Ok(s) => s,
            Err(_) => {
                let err_resp = JsonRpcResponse::err(
                    Value::Null,
                    ERR_PARSE_ERROR,
                    "Request payload is not valid UTF-8",
                    None,
                );
                if let Ok(s) = serde_json::to_string(&err_resp) {
                    writeln!(writer, "{s}")?;
                    writer.flush()?;
                }
                continue;
            }
        };

        if let Some(resp_json) = handle_jsonrpc_line(line_str) {
            writeln!(writer, "{resp_json}")?;
            writer.flush()?;
        }
    }
    Ok(())
}

/// Serves JSON-RPC 2.0 requests over `stdin` and writes responses to `stdout`.
pub fn serve_stdio() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    process_stream(stdin.lock(), stdout.lock())
}

struct ActiveClientGuard {
    sem: Arc<(Mutex<usize>, Condvar)>,
}

impl Drop for ActiveClientGuard {
    fn drop(&mut self) {
        let (lock, cvar) = &*self.sem;
        if let Ok(mut active) = lock.lock() {
            *active = active.saturating_sub(1);
            cvar.notify_one();
        }
    }
}

fn acquire_client_slot(sem: &Arc<(Mutex<usize>, Condvar)>) -> ActiveClientGuard {
    let (lock, cvar) = &**sem;
    let mut active = lock.lock().unwrap_or_else(|e| e.into_inner());
    while *active >= MAX_CONCURRENT_CLIENTS {
        active = cvar.wait(active).unwrap_or_else(|e| e.into_inner());
    }
    *active += 1;
    ActiveClientGuard {
        sem: Arc::clone(sem),
    }
}

/// Serves JSON-RPC 2.0 requests over a local Unix Domain Socket (`--socket <path>`).
/// Handles concurrent client connections via bounded scoped worker threads.
pub fn serve_unix_socket(
    socket_path: &Path,
    max_connections: Option<usize>,
) -> std::io::Result<()> {
    if socket_path.exists() {
        let _ = std::fs::remove_file(socket_path);
    }
    let listener = UnixListener::bind(socket_path)?;
    let sem = Arc::new((Mutex::new(0usize), Condvar::new()));
    let mut first_err: Option<std::io::Error> = None;

    thread::scope(|s| {
        for (accepted, stream_res) in listener.incoming().enumerate() {
            match stream_res {
                Ok(stream) => match stream.try_clone() {
                    Ok(writer) => {
                        let guard = acquire_client_slot(&sem);
                        s.spawn(move || {
                            let _guard = guard;
                            let _ = process_stream(stream, writer);
                        });
                    }
                    Err(e) => {
                        first_err = Some(e);
                        break;
                    }
                },
                Err(e) => {
                    first_err = Some(e);
                    break;
                }
            }
            if let Some(limit) = max_connections {
                if accepted + 1 >= limit {
                    break;
                }
            }
        }
    });

    let _ = std::fs::remove_file(socket_path);
    if let Some(e) = first_err {
        return Err(e);
    }
    Ok(())
}

/// Serves JSON-RPC 2.0 requests over a local TCP loopback listener (`--tcp 127.0.0.1:<port>`).
/// Handles concurrent client connections via bounded scoped worker threads.
pub fn serve_tcp(bind_addr: &str, max_connections: Option<usize>) -> std::io::Result<()> {
    let listener = TcpListener::bind(bind_addr)?;
    let sem = Arc::new((Mutex::new(0usize), Condvar::new()));

    thread::scope(|s| -> std::io::Result<()> {
        for (accepted, stream_res) in listener.incoming().enumerate() {
            let stream = stream_res?;
            let writer = stream.try_clone()?;
            let guard = acquire_client_slot(&sem);
            s.spawn(move || {
                let _guard = guard;
                let _ = process_stream(stream, writer);
            });
            if let Some(limit) = max_connections {
                if accepted + 1 >= limit {
                    break;
                }
            }
        }
        Ok(())
    })
}
