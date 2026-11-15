use proptest::prelude::*;
use titan_trit16_core::{Kleene, Trit16};

proptest! {
    #[test]
    fn interleaved_round_trip(pos in any::<u16>(), neg in any::<u16>()) {
        let original = Trit16::from_planes(pos, neg);
        let encoded = original.to_u32_interleaved();
        let decoded = Trit16::from_u32_interleaved(encoded);

        prop_assert_eq!(decoded, original);
    }

    #[test]
    fn get_matches_plane_encoding(
        pos in any::<u16>(),
        neg in any::<u16>(),
        index in 0u8..16
    ) {
        let value = Trit16::from_planes(pos, neg);
        let mask = 1u16 << index;

        let expected = match (
            (pos & mask) != 0,
            (neg & mask) != 0
        ) {
            (true, false) => Kleene::True,
            (false, true) => Kleene::False,
            (false, false) => Kleene::Unknown,
            (true, true) => Kleene::Trap,
        };

        prop_assert_eq!(value.get(index), Ok(expected));
    }

    #[test]
    fn set_updates_only_selected_trit(
        pos in any::<u16>(),
        neg in any::<u16>(),
        index in 0u8..16,
        state in 0u8..4
    ) {
        let state = match state {
            0 => Kleene::Unknown,
            1 => Kleene::True,
            2 => Kleene::False,
            _ => Kleene::Trap,
        };

        let mut value = Trit16::from_planes(pos, neg);
        let mask = 1u16 << index;
        let untouched = !mask;

        value.set(index, state).unwrap();

        prop_assert_eq!(value.get(index), Ok(state));

        prop_assert_eq!(
            value.pos() & untouched,
            pos & untouched
        );

        prop_assert_eq!(
            value.neg() & untouched,
            neg & untouched
        );
    }

    #[test]
    fn bit_plane_operations_match_specification(
        ap in any::<u16>(),
        an in any::<u16>(),
        bp in any::<u16>(),
        bn in any::<u16>()
    ) {
        let a = Trit16::from_planes(ap, an);
        let b = Trit16::from_planes(bp, bn);

        let and = a.and(b);
        prop_assert_eq!(and.pos(), ap & bp);
        prop_assert_eq!(and.neg(), an | bn);

        let or = a.or(b);
        prop_assert_eq!(or.pos(), ap | bp);
        prop_assert_eq!(or.neg(), an & bn);

        let not = a.not();
        prop_assert_eq!(not.pos(), an);
        prop_assert_eq!(not.neg(), ap);

        let collide = a.collide(b);
        let conflict = (ap & bn) | (an & bp);

        prop_assert_eq!(collide.pos(), (ap & bp) & !conflict);
        prop_assert_eq!(collide.neg(), (an & bn) & !conflict);
    }
}
