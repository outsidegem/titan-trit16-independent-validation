//! Integration and cross-layer differential tests for `titan_trit16_mcp`.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Cursor, Write};
use std::os::unix::net::UnixStream;
use std::thread;
use std::time::Duration;
use titan_trit16_core::{Kleene, Trit16};
use titan_trit16_mcp::*;

const ALL_STATES: [Kleene; 4] = [Kleene::Unknown, Kleene::True, Kleene::False, Kleene::Trap];

fn rpc_call(req: Value) -> JsonRpcResponse {
    let line = serde_json::to_string(&req).unwrap();
    let resp_str = handle_jsonrpc_line(&line).expect("Expected JSON-RPC response");
    serde_json::from_str(&resp_str).unwrap()
}

#[test]
fn test_mcp_initialize_and_tools_list() {
    let init_resp = rpc_call(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": { "protocol": "1.2" }
    }));
    assert!(init_resp.error.is_none());
    let res = init_resp.result.unwrap();
    assert_eq!(res["mcpProtocol"], "1.2");
    assert_eq!(res["serverInfo"]["name"], "titan-mcp-server");

    let list_resp = rpc_call(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    }));
    let tools = list_resp.result.unwrap()["tools"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(tools.len(), 6);
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        vec![
            "titan_encode",
            "titan_decode",
            "titan_validate",
            "titan_detect_traps",
            "titan_evaluate",
            "titan_collide"
        ]
    );
}

#[test]
fn test_all_six_tools_exhaustive_16_state_and_strict_traps() {
    let mut word_a = Trit16::zero();
    let mut word_b = Trit16::zero();
    let mut idx = 0u8;
    for &a in &ALL_STATES {
        for &b in &ALL_STATES {
            word_a.set(idx, a).unwrap();
            word_b.set(idx, b).unwrap();
            idx += 1;
        }
    }

    // 1. titan_encode & titan_decode (including all-Trap 0xFFFF)
    let enc_resp = rpc_call(json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "titan_encode",
        "params": { "pos": 0xFFFF, "neg": 0xFFFF, "strict": false }
    }));
    let enc_res = enc_resp.result.unwrap();
    assert_eq!(enc_res["pos"], 0xFFFF);
    assert_eq!(enc_res["neg"], 0xFFFF);
    assert_eq!(enc_res["interleaved"], 0xFFFF_FFFFu64);
    assert_eq!(enc_res["isValid"], false);
    assert_eq!(enc_res["hasTraps"], true);
    assert_eq!(enc_res["trapMask"], 0xFFFF);

    let dec_resp = rpc_call(json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "titan_decode",
        "params": { "interleaved": 0xFFFF_FFFFu64, "strict": false }
    }));
    let dec_res = dec_resp.result.unwrap();
    assert_eq!(dec_res["pos"], 0xFFFF);
    assert_eq!(dec_res["neg"], 0xFFFF);

    // Strict decode on 0xFFFFFFFF must return -32001 with trapMask 0xFFFF
    let strict_dec = rpc_call(json!({
        "jsonrpc": "2.0",
        "id": 12,
        "method": "titan_decode",
        "params": { "interleaved": 0xFFFF_FFFFu64, "strict": true }
    }));
    let err = strict_dec.error.unwrap();
    assert_eq!(err.code, ERR_TRAP_DETECTED);
    assert_eq!(err.data.unwrap()["trapMask"], 0xFFFF);

    // 2. titan_validate & titan_detect_traps
    let val_resp = rpc_call(json!({
        "jsonrpc": "2.0",
        "id": 13,
        "method": "titan_validate",
        "params": { "pos": word_a.pos(), "neg": word_a.neg() }
    }));
    let val_res = val_resp.result.unwrap();
    assert_eq!(val_res["isValid"], false);
    assert_eq!(val_res["trapMask"], word_a.trap_mask());

    let trap_resp = rpc_call(json!({
        "jsonrpc": "2.0",
        "id": 14,
        "method": "titan_detect_traps",
        "params": { "pos": word_a.pos(), "neg": word_a.neg(), "strict": true }
    }));
    assert_eq!(trap_resp.error.as_ref().unwrap().code, ERR_TRAP_DETECTED);
    assert_eq!(
        trap_resp.error.unwrap().data.unwrap()["trapMask"],
        word_a.trap_mask()
    );

    // 3. titan_evaluate (and, or, not, collide) vs Rust Core
    for op in ["and", "or", "collide"] {
        let eval_resp = rpc_call(json!({
            "jsonrpc": "2.0",
            "id": 20,
            "method": "tools/call",
            "params": {
                "name": "titan_evaluate",
                "arguments": {
                    "op": op,
                    "a": { "pos": word_a.pos(), "neg": word_a.neg() },
                    "b": { "pos": word_b.pos(), "neg": word_b.neg() },
                    "strict": false
                }
            }
        }));
        let sc = &eval_resp.result.unwrap()["structuredContent"];
        let expected = match op {
            "and" => word_a.and(word_b),
            "or" => word_a.or(word_b),
            "collide" => word_a.collide(word_b),
            _ => unreachable!(),
        };
        assert_eq!(sc["pos"], expected.pos());
        assert_eq!(sc["neg"], expected.neg());
        assert_eq!(sc["interleaved"], expected.to_u32_interleaved());

        // Strict variant must reject traps and report union trap mask
        let strict_resp = rpc_call(json!({
            "jsonrpc": "2.0",
            "id": 21,
            "method": "titan_evaluate",
            "params": {
                "op": op,
                "a": { "pos": word_a.pos(), "neg": word_a.neg() },
                "b": { "pos": word_b.pos(), "neg": word_b.neg() },
                "strict": true
            }
        }));
        let strict_err = strict_resp.error.unwrap();
        assert_eq!(strict_err.code, ERR_TRAP_DETECTED);
        assert_eq!(
            strict_err.data.unwrap()["trapMask"],
            word_a.trap_mask() | word_b.trap_mask()
        );
    }

    // 4. titan_collide dedicated tool
    let col_resp = rpc_call(json!({
        "jsonrpc": "2.0",
        "id": 30,
        "method": "titan_collide",
        "params": {
            "a": { "pos": word_a.pos(), "neg": word_a.neg() },
            "b": { "pos": word_b.pos(), "neg": word_b.neg() },
            "strict": false
        }
    }));
    let col_res = col_resp.result.unwrap();
    let expected_col = word_a.collide(word_b);
    assert_eq!(col_res["pos"], expected_col.pos());
    assert_eq!(col_res["neg"], expected_col.neg());

    // 5. Scalar & telemetry evaluation via titan_evaluate
    let tel_resp = rpc_call(json!({
        "jsonrpc": "2.0",
        "id": 40,
        "method": "titan_evaluate",
        "params": {
            "op": "telemetry",
            "voltageOk": "+1",
            "thermalOk": "TRAP",
            "packetOk": "-1"
        }
    }));
    assert_eq!(tel_resp.result.unwrap()["health"], "TRAP");

    // 6. titan_encode with 'trits' array and strict=true must report complete 16-bit trap mask (not just first trap)
    let mut trits_with_two_traps = vec![json!("0"); 16];
    trits_with_two_traps[2] = json!("TRAP");
    trits_with_two_traps[9] = json!("TRAP");
    let trits_strict_resp = rpc_call(json!({
        "jsonrpc": "2.0",
        "id": 41,
        "method": "titan_encode",
        "params": {
            "trits": trits_with_two_traps,
            "strict": true
        }
    }));
    let trits_err = trits_strict_resp
        .error
        .expect("Expected strict trits encode to fail on traps");
    assert_eq!(trits_err.code, ERR_TRAP_DETECTED);
    let trits_err_data = trits_err.data.unwrap();
    assert_eq!(trits_err_data["trapMask"], 0x0204);
    assert_eq!(trits_err_data["trapIndices"], json!([2, 9]));
}

#[test]
fn test_jsonrpc_envelope_utf8_and_eof_edge_cases() {
    // 1. Invalid JSON syntax -> ERR_PARSE_ERROR (-32700)
    let bad_json = handle_jsonrpc_line("{not valid json").unwrap();
    let r_bad: JsonRpcResponse = serde_json::from_str(&bad_json).unwrap();
    assert_eq!(r_bad.error.unwrap().code, ERR_PARSE_ERROR);
    assert_eq!(r_bad.id, Value::Null);

    // 2. Valid JSON non-object -> ERR_INVALID_REQUEST (-32600)
    let non_obj = handle_jsonrpc_line("[1, 2, 3]").unwrap();
    let r_non_obj: JsonRpcResponse = serde_json::from_str(&non_obj).unwrap();
    assert_eq!(r_non_obj.error.unwrap().code, ERR_INVALID_REQUEST);

    // 3. Valid JSON object missing 'method' -> ERR_INVALID_REQUEST (-32600) preserving id
    let missing_method = handle_jsonrpc_line(r#"{"jsonrpc":"2.0","id":42}"#).unwrap();
    let r_missing: JsonRpcResponse = serde_json::from_str(&missing_method).unwrap();
    assert_eq!(r_missing.error.unwrap().code, ERR_INVALID_REQUEST);
    assert_eq!(r_missing.id, json!(42));

    // 4. Non-string 'method' -> ERR_INVALID_REQUEST (-32600) preserving id
    let bad_method =
        handle_jsonrpc_line(r#"{"jsonrpc":"2.0","id":"req-43","method":123}"#).unwrap();
    let r_bad_method: JsonRpcResponse = serde_json::from_str(&bad_method).unwrap();
    assert_eq!(r_bad_method.error.unwrap().code, ERR_INVALID_REQUEST);
    assert_eq!(r_bad_method.id, json!("req-43"));

    // 5. Explicit "id": null must NOT be dropped as a notification
    let null_id_resp = handle_jsonrpc_line(r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#)
        .expect("Explicit id:null must produce a response");
    let r_null_id: JsonRpcResponse = serde_json::from_str(&null_id_resp).unwrap();
    assert!(r_null_id.error.is_none());
    assert_eq!(r_null_id.id, Value::Null);

    // 6. Omitted "id" (Notification) must return None
    assert!(
        handle_jsonrpc_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none()
    );

    // 7. Stream with invalid UTF-8 line (<= 64 KiB -> -32700), oversized invalid UTF-8 (65,537B -> -32600),
    //    and a final request without trailing newline at EOF
    let mut raw_stream = Vec::new();
    raw_stream.extend_from_slice(b"\xFF\xFE\xFD\n");
    raw_stream.extend(std::iter::repeat_n(0xFFu8, MAX_PAYLOAD_BYTES + 1));
    raw_stream.push(b'\n');
    raw_stream.extend_from_slice(br#"{"jsonrpc":"2.0","id":77,"method":"ping"}"#); // no trailing '\n'

    let mut out = Vec::new();
    process_stream(Cursor::new(raw_stream), &mut out).unwrap();
    let out_lines: Vec<&str> = std::str::from_utf8(&out).unwrap().lines().collect();
    assert_eq!(out_lines.len(), 3);

    let u0: JsonRpcResponse = serde_json::from_str(out_lines[0]).unwrap();
    assert_eq!(u0.error.unwrap().code, ERR_PARSE_ERROR);

    let u1: JsonRpcResponse = serde_json::from_str(out_lines[1]).unwrap();
    assert_eq!(u1.error.unwrap().code, ERR_INVALID_REQUEST);

    let u2: JsonRpcResponse = serde_json::from_str(out_lines[2]).unwrap();
    assert!(u2.error.is_none());
    assert_eq!(u2.id, json!(77));
}

#[test]
fn test_payload_size_limit_enforcement() {
    // 1. Exact boundary: 65,536-byte valid JSON-RPC request must succeed
    let prefix = r#"{"jsonrpc":"2.0","id":98,"method":"ping","params":{"pad":""#;
    let suffix = r#""}}"#;
    let exact_pad = "a".repeat(MAX_PAYLOAD_BYTES - prefix.len() - suffix.len());
    let exact_req = format!("{prefix}{exact_pad}{suffix}");
    assert_eq!(exact_req.len(), MAX_PAYLOAD_BYTES);

    // 2. Boundary + 1 (65,537 bytes) and oversized (66,560 bytes) must be rejected without dropping follow-up
    let plus_one_req = format!("{prefix}{exact_pad}a{suffix}");
    assert_eq!(plus_one_req.len(), MAX_PAYLOAD_BYTES + 1);
    let huge_padding = "x".repeat(MAX_PAYLOAD_BYTES + 1024);
    let valid_follow_up = r#"{"jsonrpc":"2.0","id":99,"method":"ping"}"#;
    let stream_input = format!("{exact_req}\n{plus_one_req}\n{huge_padding}\n{valid_follow_up}\n");

    let mut output = Vec::new();
    process_stream(Cursor::new(stream_input.into_bytes()), &mut output).unwrap();

    let out_str = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = out_str.lines().collect();
    assert_eq!(lines.len(), 4);

    let r0: JsonRpcResponse = serde_json::from_str(lines[0]).unwrap();
    assert!(r0.error.is_none());
    assert_eq!(r0.id, json!(98));

    let r1: JsonRpcResponse = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(r1.error.unwrap().code, ERR_INVALID_REQUEST);

    let r2: JsonRpcResponse = serde_json::from_str(lines[2]).unwrap();
    assert_eq!(r2.error.unwrap().code, ERR_INVALID_REQUEST);

    let r3: JsonRpcResponse = serde_json::from_str(lines[3]).unwrap();
    assert!(r3.error.is_none());
    assert_eq!(r3.id, json!(99));
}

#[test]
fn test_unix_domain_socket_transport() {
    let sock_path =
        std::env::temp_dir().join(format!("titan_mcp_test_{}.sock", std::process::id()));
    if sock_path.exists() {
        let _ = std::fs::remove_file(&sock_path);
    }

    let server_sock = sock_path.clone();
    let server_handle = thread::spawn(move || {
        serve_unix_socket(&server_sock, Some(2)).unwrap();
    });

    // Wait briefly for listener bind
    for _ in 0..50 {
        if sock_path.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    // Client 1 connects and stays idle (does not send EOF) while Client 2 connects and completes a request
    let mut client1 =
        UnixStream::connect(&sock_path).expect("Failed to connect client 1 to Unix socket");
    let mut client2 =
        UnixStream::connect(&sock_path).expect("Failed to connect client 2 to Unix socket");

    let req2 =
        r#"{"jsonrpc":"2.0","id":702,"method":"titan_encode","params":{"pos":15,"neg":240}}"#;
    writeln!(client2, "{req2}").unwrap();
    client2.flush().unwrap();

    let mut reader2 = BufReader::new(client2);
    let mut resp_line2 = String::new();
    reader2.read_line(&mut resp_line2).unwrap();
    let resp2: JsonRpcResponse = serde_json::from_str(&resp_line2).unwrap();
    assert!(resp2.error.is_none());
    assert_eq!(resp2.id, json!(702));
    assert_eq!(resp2.result.unwrap()["pos"], 15);
    drop(reader2);

    let req1 =
        r#"{"jsonrpc":"2.0","id":7,"method":"titan_encode","params":{"pos":255,"neg":3840}}"#;
    writeln!(client1, "{req1}").unwrap();
    client1.flush().unwrap();

    let mut reader1 = BufReader::new(client1);
    let mut resp_line1 = String::new();
    reader1.read_line(&mut resp_line1).unwrap();

    let resp1: JsonRpcResponse = serde_json::from_str(&resp_line1).unwrap();
    assert!(resp1.error.is_none());
    let res = resp1.result.unwrap();
    assert_eq!(res["pos"], 255);
    assert_eq!(res["neg"], 3840);
    assert_eq!(res["isValid"], true);

    drop(reader1);
    server_handle.join().unwrap();
}

#[test]
fn test_tcp_loopback_transport() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    drop(listener);

    let server_addr = addr.clone();
    let server_handle = thread::spawn(move || {
        serve_tcp(&server_addr, Some(2)).unwrap();
    });

    let mut client1 = None;
    for _ in 0..50 {
        if let Ok(stream) = std::net::TcpStream::connect(&addr) {
            client1 = Some(stream);
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    let mut client1 = client1.expect("Failed to connect client 1 to TCP loopback server");
    let mut client2 = std::net::TcpStream::connect(&addr)
        .expect("Failed to connect client 2 to TCP loopback server");

    // Client 2 sends and receives while Client 1 holds its socket open
    let req2 = r#"{"jsonrpc":"2.0","id":802,"method":"ping"}"#;
    writeln!(client2, "{req2}").unwrap();
    client2.flush().unwrap();
    let mut reader2 = BufReader::new(client2);
    let mut resp_line2 = String::new();
    reader2.read_line(&mut resp_line2).unwrap();
    let resp2: JsonRpcResponse = serde_json::from_str(&resp_line2).unwrap();
    assert!(resp2.error.is_none());
    assert_eq!(resp2.id, json!(802));
    drop(reader2);

    let req1 =
        r#"{"jsonrpc":"2.0","id":8,"method":"titan_decode","params":{"interleaved":2863289685}}"#;
    writeln!(client1, "{req1}").unwrap();
    client1.flush().unwrap();

    let mut reader1 = BufReader::new(client1);
    let mut resp_line1 = String::new();
    reader1.read_line(&mut resp_line1).unwrap();

    let resp1: JsonRpcResponse = serde_json::from_str(&resp_line1).unwrap();
    assert!(resp1.error.is_none());
    let res = resp1.result.unwrap();
    assert_eq!(res["pos"], 255);
    assert_eq!(res["neg"], 65280);
    assert_eq!(res["isValid"], true);

    drop(reader1);
    server_handle.join().unwrap();
}
