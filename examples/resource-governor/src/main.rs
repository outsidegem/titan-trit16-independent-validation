//! Monitor-only ZTPP Telemetry Homeostasis Governor Demonstration (`titan-resource-governor`).
//!
//! Operates strictly in read-only, non-root, deterministic mode without network
//! or privileged kernel calls.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use titan_trit16_core::{Kleene, Trit16, TritError};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernorReport {
    pub scenario: String,
    pub scalar_health: String,
    pub primary_bank: String,
    pub secondary_bank: String,
    pub consensus_bank: String,
    pub strict_passed: bool,
    pub detected_trap_mask: u16,
    pub recommended_action: String,
}

pub fn evaluate_homeostasis(
    scenario: &str,
    voltage_ok: Kleene,
    thermal_ok: Kleene,
    packet_ok: Kleene,
    primary_bank: Trit16,
    secondary_bank: Trit16,
) -> GovernorReport {
    let health = Kleene::evaluate_telemetry(voltage_ok, thermal_ok, packet_ok);
    let (consensus, strict_passed, trap_mask) = match primary_bank.collide_strict(secondary_bank) {
        Ok(c) => (c, true, 0u16),
        Err(TritError::TrapDetected(mask)) => (primary_bank.collide(secondary_bank), false, mask),
        Err(TritError::InvalidIndex(_)) => unreachable!(),
    };

    let recommended_action = match (health, strict_passed) {
        (Kleene::Trap, _) | (_, false) => {
            format!("ISOLATE_SENSOR_FAULT (trap_mask=0x{trap_mask:04X}) & MAINTAIN_SAFE_CLOCK")
        }
        (Kleene::True, true) => "NOMINAL_OPERATION".to_string(),
        (Kleene::Unknown, true) => {
            "HOLD_CURRENT_OPERATING_POINT (awaiting sensor convergence)".to_string()
        }
        (Kleene::False, true) => "SHED_NON_CRITICAL_LOAD (thermal/voltage constraint)".to_string(),
    };

    GovernorReport {
        scenario: scenario.to_string(),
        scalar_health: health.to_string(),
        primary_bank: primary_bank.to_string(),
        secondary_bank: secondary_bank.to_string(),
        consensus_bank: consensus.to_string(),
        strict_passed,
        detected_trap_mask: trap_mask,
        recommended_action,
    }
}

fn main() {
    let scenarios = [
        evaluate_homeostasis(
            "1_nominal_homeostasis",
            Kleene::True,
            Kleene::True,
            Kleene::True,
            Trit16::from_planes(0x00FF, 0xFF00),
            Trit16::from_planes(0x000F, 0xF000),
        ),
        evaluate_homeostasis(
            "2_thermal_constraint",
            Kleene::True,
            Kleene::False,
            Kleene::True,
            Trit16::from_planes(0x00FF, 0xFF00),
            Trit16::from_planes(0x00FF, 0xFF00),
        ),
        evaluate_homeostasis(
            "3_hardware_sensor_trap_fault",
            Kleene::True,
            Kleene::Trap,
            Kleene::True,
            Trit16::from_planes(0x0808, 0x0808),
            Trit16::from_planes(0x00FF, 0xFF00),
        ),
    ];

    println!("{}", serde_json::to_string_pretty(&scenarios).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_homeostasis_all_four_regimes() {
        let clean_a = Trit16::from_planes(0x00FF, 0xFF00);
        let clean_b = Trit16::from_planes(0x000F, 0xF000);

        let r_nom = evaluate_homeostasis(
            "nominal",
            Kleene::True,
            Kleene::True,
            Kleene::True,
            clean_a,
            clean_b,
        );
        assert_eq!(r_nom.scalar_health, "+1");
        assert!(r_nom.strict_passed);
        assert_eq!(r_nom.detected_trap_mask, 0);
        assert_eq!(r_nom.recommended_action, "NOMINAL_OPERATION");

        let r_unk = evaluate_homeostasis(
            "unknown",
            Kleene::True,
            Kleene::Unknown,
            Kleene::True,
            clean_a,
            clean_b,
        );
        assert_eq!(r_unk.scalar_health, "0");
        assert!(r_unk.strict_passed);
        assert!(r_unk
            .recommended_action
            .starts_with("HOLD_CURRENT_OPERATING_POINT"));

        let r_shed = evaluate_homeostasis(
            "shed",
            Kleene::True,
            Kleene::False,
            Kleene::True,
            clean_a,
            clean_b,
        );
        assert_eq!(r_shed.scalar_health, "-1");
        assert!(r_shed.strict_passed);
        assert!(r_shed
            .recommended_action
            .starts_with("SHED_NON_CRITICAL_LOAD"));

        let trapped_a = Trit16::from_planes(0x0808, 0x0808);
        let r_trap = evaluate_homeostasis(
            "trap",
            Kleene::True,
            Kleene::True,
            Kleene::True,
            trapped_a,
            clean_b,
        );
        assert!(!r_trap.strict_passed);
        assert_eq!(r_trap.detected_trap_mask, 0x0808);
        assert!(r_trap.recommended_action.contains("ISOLATE_SENSOR_FAULT"));
    }
}
