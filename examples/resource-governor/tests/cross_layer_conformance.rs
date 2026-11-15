//! Cross-layer conformance test suite verifying `tests/vectors.json` across:
//! 1. Rust Core (`titan_trit16_core`)
//! 2. C ABI (`titan_trit16_ffi`)
//! 3. JNI Native Bridge (`titan_trit16_ffi::jni`)
//! 4. MCP JSON-RPC 2.0 Server (`titan_trit16_mcp`)

use core::ffi::c_void;
use serde_json::{json, Value};
use titan_trit16_core::{Trit16, TritError};
use titan_trit16_ffi::jni::*;
use titan_trit16_ffi::*;
use titan_trit16_mcp::{dispatch_tool, ERR_TRAP_DETECTED};

#[test]
fn test_shared_conformance_vectors_across_all_layers() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let vectors_path = format!("{manifest_dir}/../../tests/vectors.json");
    let raw_json =
        std::fs::read_to_string(&vectors_path).expect("Failed to read tests/vectors.json");
    let doc: Value = serde_json::from_str(&raw_json).expect("Invalid JSON in vectors.json");
    let vectors = doc["vectors"].as_array().expect("Expected vectors array");
    assert_eq!(vectors.len(), 4);

    let env: *mut c_void = core::ptr::null_mut();
    let cls: *mut c_void = core::ptr::null_mut();

    for vec_entry in vectors {
        let name = vec_entry["name"].as_str().unwrap();
        let a_pos = vec_entry["a"]["pos"].as_u64().unwrap() as u16;
        let a_neg = vec_entry["a"]["neg"].as_u64().unwrap() as u16;
        let a_interleaved = vec_entry["a"]["interleaved"].as_u64().unwrap() as u32;
        let a_trap_mask = vec_entry["a"]["trapMask"].as_u64().unwrap() as u16;

        let b_pos = vec_entry["b"]["pos"].as_u64().unwrap() as u16;
        let b_neg = vec_entry["b"]["neg"].as_u64().unwrap() as u16;
        let b_interleaved = vec_entry["b"]["interleaved"].as_u64().unwrap() as u32;
        let b_trap_mask = vec_entry["b"]["trapMask"].as_u64().unwrap() as u16;

        let exp = &vec_entry["expected"];
        let exp_and_pos = exp["and"]["pos"].as_u64().unwrap() as u16;
        let exp_and_neg = exp["and"]["neg"].as_u64().unwrap() as u16;
        let exp_or_pos = exp["or"]["pos"].as_u64().unwrap() as u16;
        let exp_or_neg = exp["or"]["neg"].as_u64().unwrap() as u16;
        let exp_not_pos = exp["notA"]["pos"].as_u64().unwrap() as u16;
        let exp_not_neg = exp["notA"]["neg"].as_u64().unwrap() as u16;
        let exp_col_pos = exp["collide"]["pos"].as_u64().unwrap() as u16;
        let exp_col_neg = exp["collide"]["neg"].as_u64().unwrap() as u16;
        let exp_strict_valid = exp["strictValid"].as_bool().unwrap();
        let exp_union_mask = exp["unionTrapMask"].as_u64().unwrap() as u16;

        // 1. Rust Core verification
        let ra = Trit16::from_planes(a_pos, a_neg);
        let rb = Trit16::from_planes(b_pos, b_neg);
        assert_eq!(
            ra.to_u32_interleaved(),
            a_interleaved,
            "Core interleaved A ({name})"
        );
        assert_eq!(
            rb.to_u32_interleaved(),
            b_interleaved,
            "Core interleaved B ({name})"
        );
        assert_eq!(
            Trit16::from_u32_interleaved(a_interleaved),
            ra,
            "Core from_u32 A ({name})"
        );
        assert_eq!(
            Trit16::from_u32_interleaved(b_interleaved),
            rb,
            "Core from_u32 B ({name})"
        );
        assert_eq!(ra.trap_mask(), a_trap_mask, "Core trap_mask A ({name})");
        assert_eq!(rb.trap_mask(), b_trap_mask, "Core trap_mask B ({name})");
        assert_eq!(
            ra.and(rb),
            Trit16::from_planes(exp_and_pos, exp_and_neg),
            "Core AND ({name})"
        );
        assert_eq!(
            ra.or(rb),
            Trit16::from_planes(exp_or_pos, exp_or_neg),
            "Core OR ({name})"
        );
        assert_eq!(
            ra.not(),
            Trit16::from_planes(exp_not_pos, exp_not_neg),
            "Core NOT ({name})"
        );
        assert_eq!(
            ra.collide(rb),
            Trit16::from_planes(exp_col_pos, exp_col_neg),
            "Core COLLIDE ({name})"
        );

        // 2. C ABI verification
        let ca = titan_trit16_from_planes(a_pos, a_neg);
        let cb = titan_trit16_from_planes(b_pos, b_neg);
        assert_eq!(
            titan_trit16_to_u32_interleaved(ca),
            a_interleaved,
            "FFI interleaved A ({name})"
        );
        assert_eq!(
            titan_trit16_to_u32_interleaved(cb),
            b_interleaved,
            "FFI interleaved B ({name})"
        );
        assert_eq!(
            titan_trit16_from_u32_interleaved(a_interleaved),
            ca,
            "FFI from_u32 A ({name})"
        );
        assert_eq!(
            titan_trit16_from_u32_interleaved(b_interleaved),
            cb,
            "FFI from_u32 B ({name})"
        );
        assert_eq!(
            titan_trit16_trap_mask(ca),
            a_trap_mask,
            "FFI trap_mask A ({name})"
        );
        assert_eq!(
            titan_trit16_trap_mask(cb),
            b_trap_mask,
            "FFI trap_mask B ({name})"
        );
        assert_eq!(
            titan_trit16_and(ca, cb),
            titan_trit16_from_planes(exp_and_pos, exp_and_neg)
        );
        assert_eq!(
            titan_trit16_or(ca, cb),
            titan_trit16_from_planes(exp_or_pos, exp_or_neg)
        );
        assert_eq!(
            titan_trit16_not(ca),
            titan_trit16_from_planes(exp_not_pos, exp_not_neg)
        );
        assert_eq!(
            titan_trit16_collide(ca, cb),
            titan_trit16_from_planes(exp_col_pos, exp_col_neg)
        );

        let mut c_out = titan_trit16_zero();
        let mut c_mask = 0u16;
        let c_strict_status =
            unsafe { titan_trit16_collide_strict(ca, cb, &mut c_out, &mut c_mask) };
        if exp_strict_valid {
            assert_eq!(c_strict_status, TitanError::Ok);
            assert_eq!(c_mask, 0);
            assert_eq!(c_out, titan_trit16_from_planes(exp_col_pos, exp_col_neg));
        } else {
            assert_eq!(c_strict_status, TitanError::TrapDetected);
            assert_eq!(c_mask, exp_union_mask);
        }

        // 3. JNI Native Bridge verification
        let ja =
            Java_com_titan_trit16_Trit16Native_fromPlanes(env, cls, a_pos as i32, a_neg as i32);
        let jb =
            Java_com_titan_trit16_Trit16Native_fromPlanes(env, cls, b_pos as i32, b_neg as i32);
        assert_eq!(
            Java_com_titan_trit16_Trit16Native_toU32Interleaved(env, cls, ja) as u32,
            a_interleaved,
            "JNI interleaved A ({name})"
        );
        assert_eq!(
            Java_com_titan_trit16_Trit16Native_fromU32Interleaved(env, cls, a_interleaved as i32),
            ja,
            "JNI fromU32Interleaved A ({name})"
        );
        assert_eq!(
            Java_com_titan_trit16_Trit16Native_trapMask(env, cls, ja) as u16,
            a_trap_mask,
            "JNI trapMask A ({name})"
        );
        assert_eq!(
            Java_com_titan_trit16_Trit16Native_andPlanes(env, cls, ja, jb),
            Java_com_titan_trit16_Trit16Native_fromPlanes(
                env,
                cls,
                exp_and_pos as i32,
                exp_and_neg as i32
            )
        );
        assert_eq!(
            Java_com_titan_trit16_Trit16Native_orPlanes(env, cls, ja, jb),
            Java_com_titan_trit16_Trit16Native_fromPlanes(
                env,
                cls,
                exp_or_pos as i32,
                exp_or_neg as i32
            )
        );
        assert_eq!(
            Java_com_titan_trit16_Trit16Native_notPlanes(env, cls, ja),
            Java_com_titan_trit16_Trit16Native_fromPlanes(
                env,
                cls,
                exp_not_pos as i32,
                exp_not_neg as i32
            )
        );
        assert_eq!(
            Java_com_titan_trit16_Trit16Native_collidePlanes(env, cls, ja, jb),
            Java_com_titan_trit16_Trit16Native_fromPlanes(
                env,
                cls,
                exp_col_pos as i32,
                exp_col_neg as i32
            )
        );
        let jni_col_strict = Java_com_titan_trit16_Trit16Native_collideStrict(env, cls, ja, jb);
        if exp_strict_valid {
            assert_eq!((jni_col_strict >> 48) & 0xFF, TitanError::Ok as i64);
            assert_eq!(
                (jni_col_strict & 0xFFFF_FFFF) as i32,
                Java_com_titan_trit16_Trit16Native_fromPlanes(
                    env,
                    cls,
                    exp_col_pos as i32,
                    exp_col_neg as i32
                )
            );
        } else {
            assert_eq!(
                (jni_col_strict >> 48) & 0xFF,
                TitanError::TrapDetected as i64
            );
            assert_eq!(((jni_col_strict >> 32) & 0xFFFF) as u16, exp_union_mask);
        }

        // 4. MCP JSON-RPC Tool verification across all 6 tools
        let mcp_enc = dispatch_tool(
            "titan_encode",
            &json!({ "pos": a_pos, "neg": a_neg, "strict": false }),
        )
        .unwrap();
        assert_eq!(
            mcp_enc["interleaved"], a_interleaved,
            "MCP encode interleaved A ({name})"
        );
        assert_eq!(
            mcp_enc["trapMask"], a_trap_mask,
            "MCP encode trapMask A ({name})"
        );
        assert_eq!(
            mcp_enc["display"].as_str().unwrap(),
            ra.to_string(),
            "MCP display A ({name})"
        );

        let mcp_dec = dispatch_tool(
            "titan_decode",
            &json!({ "interleaved": a_interleaved, "strict": false }),
        )
        .unwrap();
        assert_eq!(mcp_dec["pos"], a_pos, "MCP decode pos A ({name})");
        assert_eq!(mcp_dec["neg"], a_neg, "MCP decode neg A ({name})");

        let mcp_val =
            dispatch_tool("titan_validate", &json!({ "pos": a_pos, "neg": a_neg })).unwrap();
        assert_eq!(
            mcp_val["isValid"],
            a_trap_mask == 0,
            "MCP validate A ({name})"
        );
        assert_eq!(
            mcp_val["trapMask"], a_trap_mask,
            "MCP validate trapMask A ({name})"
        );

        let mcp_traps = dispatch_tool(
            "titan_detect_traps",
            &json!({ "pos": a_pos, "neg": a_neg, "strict": false }),
        )
        .unwrap();
        assert_eq!(
            mcp_traps["trapMask"], a_trap_mask,
            "MCP detect_traps A ({name})"
        );

        for (op, (ep, en)) in [
            ("and", (exp_and_pos, exp_and_neg)),
            ("or", (exp_or_pos, exp_or_neg)),
            ("collide", (exp_col_pos, exp_col_neg)),
        ] {
            let eval_res = dispatch_tool(
                "titan_evaluate",
                &json!({
                    "op": op,
                    "a": { "pos": a_pos, "neg": a_neg },
                    "b": { "pos": b_pos, "neg": b_neg },
                    "strict": false
                }),
            )
            .unwrap();
            assert_eq!(eval_res["pos"], ep, "MCP evaluate {op} pos ({name})");
            assert_eq!(eval_res["neg"], en, "MCP evaluate {op} neg ({name})");
        }

        let eval_not = dispatch_tool(
            "titan_evaluate",
            &json!({
                "op": "not",
                "a": { "pos": a_pos, "neg": a_neg },
                "strict": false
            }),
        )
        .unwrap();
        assert_eq!(
            eval_not["pos"], exp_not_pos,
            "MCP evaluate not pos ({name})"
        );
        assert_eq!(
            eval_not["neg"], exp_not_neg,
            "MCP evaluate not neg ({name})"
        );

        let mcp_col = dispatch_tool(
            "titan_collide",
            &json!({
                "a": { "pos": a_pos, "neg": a_neg },
                "b": { "pos": b_pos, "neg": b_neg },
                "strict": false
            }),
        )
        .unwrap();
        assert_eq!(mcp_col["pos"], exp_col_pos, "MCP collide pos ({name})");
        assert_eq!(mcp_col["neg"], exp_col_neg, "MCP collide neg ({name})");

        let mcp_strict = dispatch_tool(
            "titan_collide",
            &json!({
                "a": { "pos": a_pos, "neg": a_neg },
                "b": { "pos": b_pos, "neg": b_neg },
                "strict": true
            }),
        );
        if exp_strict_valid {
            let val = mcp_strict.expect("Expected strict MCP collide to succeed");
            assert_eq!(val["pos"], exp_col_pos);
            assert_eq!(val["neg"], exp_col_neg);
            assert_eq!(
                ra.collide_strict(rb).unwrap(),
                Trit16::from_planes(exp_col_pos, exp_col_neg)
            );
        } else {
            let err = mcp_strict.expect_err("Expected strict MCP collide to fail on traps");
            assert_eq!(err.code, ERR_TRAP_DETECTED);
            assert_eq!(err.data.unwrap()["trapMask"], exp_union_mask);
            assert_eq!(
                ra.collide_strict(rb),
                Err(TritError::TrapDetected(exp_union_mask))
            );
        }
    }
}
