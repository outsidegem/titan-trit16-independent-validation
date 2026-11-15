// Property‑based tests for Titan Trit16 core

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use titan_trit16_core::{trit16, Kleene, Trit16};

    // Reference encoder/decoder that does not reuse internal impl
    fn ref_encode(trit: &Trit16) -> Vec<u8> {
        // Simple direct bit‑plane serialization for testing
        let mut bytes = Vec::new();
        for plane in 0..16 {
            let mut byte: u8 = 0;
            for bit in 0..8 {
                let idx = plane * 8 + bit;
                let val = trit.get_bit(idx).unwrap_or(0);
                byte |= (val << bit);
            }
            bytes.push(byte);
        }
        bytes
    }

    fn ref_decode(bytes: &[u8]) -> Option<Trit16> {
        if bytes.len() != 16 { return None; }
        let mut trit = Trit16::new();
        for plane in 0..16 {
            let byte = bytes[plane];
            for bit in 0..8 {
                let idx = plane * 8 + bit;
                let val = (byte >> bit) & 1;
                trit.set_bit(idx, val).ok()?;
            }
        }
        Some(trit)
    }

    proptest! {
        #[test]
        fn round_trip_random(bits in proptest::collection::vec(0u8..=1, 128)) {
            // Build a Trit16 from random bits
            let mut trit = Trit16::new();
            for (i, b) in bits.iter().enumerate() {
                trit.set_bit(i, *b).unwrap();
            }
            let encoded = ref_encode(&trit);
            // Use production encode if exists
            let prod_enc = trit16::encode(&trit).expect("encode should succeed");
            prop_assert_eq!(encoded, prod_enc);
            // Decode using reference and production
            let decoded_ref = ref_decode(&encoded).expect("ref decode");
            let decoded_prod = trit16::decode(&encoded).expect("prod decode");
            prop_assert_eq!(decoded_ref, decoded_prod);
        }

        #[test]
        fn kleene_states(k in any::<Kleene>()) {
            // Verify that encoding a single‑state Kleene behaves as spec
            let trit = Trit16::from_kleene(k);
            let enc = trit16::encode(&trit).unwrap();
            let dec = trit16::decode(&enc).unwrap();
            prop_assert_eq!(trit, dec);
        }

        #[test]
        fn logical_ops(a in any::<Trit16>(), b in any::<Trit16>()) {
            // AND, OR, NOT, COLLIDE semantics
            let and = a.and(&b);
            let or = a.or(&b);
            let not_a = a.not();
            let collide = a.collide(&b);
            // Encode/decode round‑trip for each
            for op in &[and, or, not_a, collide] {
                let enc = trit16::encode(op).unwrap();
                let dec = trit16::decode(&enc).unwrap();
                prop_assert_eq!(op, &dec);
            }
        }
    }
}
