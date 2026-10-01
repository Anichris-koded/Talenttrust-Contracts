#![cfg(test)]
//! Focused validation-boundary tests for `contracts/escrow/src/events.rs`.
//!
//! Covers every emit function with:
//!  - accepted (valid) inputs
//!  - rejected inputs (zero contract_id, negative amounts, empty/too-long evidence, out-of-bounds milestone index)
//!  - exact-boundary values (id == 1, id == u32::MAX, index == MAX_MILESTONES - 1, amount == 0, evidence == 1 byte, evidence == MAX_WORK_EVIDENCE_BYTES)
//!  - regression guard for validate_event_amounts used internally

extern crate std;

use crate::events::{
    emit_contract_indexed_event, emit_dispute_opened_event, emit_dispute_resolved_event,
    emit_milestone_approved_event, emit_milestone_refunded_event, emit_milestone_released_event,
    emit_work_evidence_submitted_event, validate_event_amounts,
};
use crate::milestones_consts::{MAX_MILESTONES, MAX_WORK_EVIDENCE_BYTES};
use crate::types::ContractStatus;
use crate::EscrowError;
use soroban_sdk::testutils::{Address as _, Events};
use soroban_sdk::{Env, String};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn zero_contract() -> crate::Contract {
    crate::Contract {
        status: ContractStatus::Created,
        funded_amount: 0,
        released_amount: 0,
        refunded_amount: 0,
        total_deposited: 0,
        ..Default::default()
    }
}

fn funded_contract() -> crate::Contract {
    crate::Contract {
        status: ContractStatus::Funded,
        funded_amount: 1_000,
        released_amount: 0,
        refunded_amount: 0,
        total_deposited: 1_000,
        ..Default::default()
    }
}

// ── validate_event_amounts (unit-level regression) ────────────────────────────

#[test]
fn validate_amounts_accepts_all_zero() {
    assert_eq!(validate_event_amounts(0, 0, 0, 0), Ok(()));
}

#[test]
fn validate_amounts_accepts_typical_values() {
    assert_eq!(validate_event_amounts(1_000, 500, 200, 1_000), Ok(()));
}

#[test]
fn validate_amounts_accepts_i128_max() {
    assert_eq!(validate_event_amounts(i128::MAX, 0, 0, 0), Ok(()));
    assert_eq!(validate_event_amounts(0, i128::MAX, 0, 0), Ok(()));
    assert_eq!(validate_event_amounts(0, 0, i128::MAX, 0), Ok(()));
    assert_eq!(validate_event_amounts(0, 0, 0, i128::MAX), Ok(()));
}

#[test]
fn validate_amounts_rejects_negative_funded() {
    assert_eq!(
        validate_event_amounts(-1, 0, 0, 0),
        Err(EscrowError::AmountMustBePositive)
    );
}

#[test]
fn validate_amounts_rejects_negative_released() {
    assert_eq!(
        validate_event_amounts(0, -1, 0, 0),
        Err(EscrowError::AmountMustBePositive)
    );
}

#[test]
fn validate_amounts_rejects_negative_refunded() {
    assert_eq!(
        validate_event_amounts(0, 0, -1, 0),
        Err(EscrowError::AmountMustBePositive)
    );
}

#[test]
fn validate_amounts_rejects_negative_total_deposited() {
    assert_eq!(
        validate_event_amounts(0, 0, 0, -1),
        Err(EscrowError::AmountMustBePositive)
    );
}

// ── emit_contract_indexed_event ───────────────────────────────────────────────

#[test]
fn contract_indexed_accepts_id_one() {
    let env = env();
    emit_contract_indexed_event(&env, 1, &zero_contract());
    assert!(!env.events().all().is_empty());
}

#[test]
fn contract_indexed_accepts_id_max() {
    let env = env();
    emit_contract_indexed_event(&env, u32::MAX, &zero_contract());
    assert!(!env.events().all().is_empty());
}

#[test]
#[should_panic]
fn contract_indexed_rejects_zero_id() {
    let env = env();
    emit_contract_indexed_event(&env, 0, &zero_contract());
}

#[test]
#[should_panic]
fn contract_indexed_rejects_negative_amounts() {
    let env = env();
    let bad = crate::Contract {
        status: ContractStatus::Created,
        funded_amount: -1,
        released_amount: 0,
        refunded_amount: 0,
        total_deposited: 0,
        ..Default::default()
    };
    emit_contract_indexed_event(&env, 1, &bad);
}

#[test]
fn contract_indexed_emits_for_every_status() {
    let statuses = [
        ContractStatus::Created,
        ContractStatus::Funded,
        ContractStatus::Completed,
        ContractStatus::Disputed,
        ContractStatus::Cancelled,
        ContractStatus::Refunded,
        ContractStatus::PartiallyFunded,
    ];
    for status in statuses {
        let env = env();
        let c = crate::Contract {
            status,
            funded_amount: 100,
            released_amount: 50,
            refunded_amount: 10,
            total_deposited: 100,
            ..Default::default()
        };
        emit_contract_indexed_event(&env, 1, &c);
        assert!(
            !env.events().all().is_empty(),
            "must emit for status {status:?}"
        );
    }
}

// ── emit_dispute_opened_event ─────────────────────────────────────────────────

#[test]
fn dispute_opened_accepts_valid() {
    let env = env();
    let caller = soroban_sdk::Address::generate(&env);
    emit_dispute_opened_event(&env, 1, &caller, &funded_contract());
    assert!(!env.events().all().is_empty());
}

#[test]
fn dispute_opened_accepts_id_boundary() {
    let env = env();
    let caller = soroban_sdk::Address::generate(&env);
    emit_dispute_opened_event(&env, u32::MAX, &caller, &funded_contract());
    assert!(!env.events().all().is_empty());
}

#[test]
#[should_panic]
fn dispute_opened_rejects_zero_id() {
    let env = env();
    let caller = soroban_sdk::Address::generate(&env);
    emit_dispute_opened_event(&env, 0, &caller, &funded_contract());
}

#[test]
#[should_panic]
fn dispute_opened_rejects_negative_funded_amount() {
    let env = env();
    let caller = soroban_sdk::Address::generate(&env);
    let bad = crate::Contract {
        status: ContractStatus::Funded,
        funded_amount: -1,
        released_amount: 0,
        refunded_amount: 0,
        total_deposited: 0,
        ..Default::default()
    };
    emit_dispute_opened_event(&env, 1, &caller, &bad);
}

#[test]
#[should_panic]
fn dispute_opened_rejects_negative_released_amount() {
    let env = env();
    let caller = soroban_sdk::Address::generate(&env);
    let bad = crate::Contract {
        status: ContractStatus::Disputed,
        funded_amount: 500,
        released_amount: -1,
        refunded_amount: 0,
        total_deposited: 500,
        ..Default::default()
    };
    emit_dispute_opened_event(&env, 1, &caller, &bad);
}

// ── emit_dispute_resolved_event ───────────────────────────────────────────────

#[test]
fn dispute_resolved_accepts_valid() {
    let env = env();
    emit_dispute_resolved_event(&env, 1, 400, 300, 0, ContractStatus::Completed);
    assert!(!env.events().all().is_empty());
}

#[test]
fn dispute_resolved_accepts_zero_payouts() {
    let env = env();
    emit_dispute_resolved_event(&env, 1, 0, 0, 0, ContractStatus::Disputed);
    assert!(!env.events().all().is_empty());
}

#[test]
fn dispute_resolved_accepts_id_boundary() {
    let env = env();
    emit_dispute_resolved_event(&env, u32::MAX, 100, 200, 1, ContractStatus::Completed);
    assert!(!env.events().all().is_empty());
}

#[test]
#[should_panic]
fn dispute_resolved_rejects_zero_id() {
    let env = env();
    emit_dispute_resolved_event(&env, 0, 100, 200, 0, ContractStatus::Completed);
}

#[test]
#[should_panic]
fn dispute_resolved_rejects_negative_client_payout() {
    let env = env();
    emit_dispute_resolved_event(&env, 1, -1, 300, 0, ContractStatus::Completed);
}

#[test]
#[should_panic]
fn dispute_resolved_rejects_negative_freelancer_payout() {
    let env = env();
    emit_dispute_resolved_event(&env, 1, 400, -1, 0, ContractStatus::Completed);
}

#[test]
#[should_panic]
fn dispute_resolved_rejects_both_payouts_negative() {
    let env = env();
    emit_dispute_resolved_event(&env, 1, -100, -200, 0, ContractStatus::Completed);
}

// ── emit_milestone_released_event ────────────────────────────────────────────

#[test]
fn milestone_released_accepts_valid() {
    let env = env();
    let recipient = soroban_sdk::Address::generate(&env);
    emit_milestone_released_event(&env, 1, 0, 900, 1_000, 100, &recipient);
    assert!(!env.events().all().is_empty());
}

#[test]
fn milestone_released_accepts_zero_fee() {
    let env = env();
    let recipient = soroban_sdk::Address::generate(&env);
    emit_milestone_released_event(&env, 1, 0, 1_000, 1_000, 0, &recipient);
    assert!(!env.events().all().is_empty());
}

#[test]
fn milestone_released_accepts_max_index() {
    let env = env();
    let recipient = soroban_sdk::Address::generate(&env);
    // MAX_MILESTONES - 1 is the highest valid index
    emit_milestone_released_event(&env, 1, MAX_MILESTONES - 1, 500, 500, 0, &recipient);
    assert!(!env.events().all().is_empty());
}

#[test]
fn milestone_released_accepts_id_boundary() {
    let env = env();
    let recipient = soroban_sdk::Address::generate(&env);
    emit_milestone_released_event(&env, u32::MAX, 0, 500, 500, 0, &recipient);
    assert!(!env.events().all().is_empty());
}

#[test]
#[should_panic]
fn milestone_released_rejects_zero_contract_id() {
    let env = env();
    let recipient = soroban_sdk::Address::generate(&env);
    emit_milestone_released_event(&env, 0, 0, 900, 1_000, 100, &recipient);
}

#[test]
#[should_panic]
fn milestone_released_rejects_out_of_bounds_index() {
    let env = env();
    let recipient = soroban_sdk::Address::generate(&env);
    emit_milestone_released_event(&env, 1, MAX_MILESTONES, 900, 1_000, 100, &recipient);
}

#[test]
#[should_panic]
fn milestone_released_rejects_negative_amount() {
    let env = env();
    let recipient = soroban_sdk::Address::generate(&env);
    emit_milestone_released_event(&env, 1, 0, -1, 1_000, 100, &recipient);
}

#[test]
#[should_panic]
fn milestone_released_rejects_negative_gross_amount() {
    let env = env();
    let recipient = soroban_sdk::Address::generate(&env);
    emit_milestone_released_event(&env, 1, 0, 900, -1, 100, &recipient);
}

#[test]
#[should_panic]
fn milestone_released_rejects_negative_fee() {
    let env = env();
    let recipient = soroban_sdk::Address::generate(&env);
    emit_milestone_released_event(&env, 1, 0, 900, 1_000, -1, &recipient);
}

// ── emit_milestone_refunded_event ─────────────────────────────────────────────

#[test]
fn milestone_refunded_accepts_valid() {
    let env = env();
    let recipient = soroban_sdk::Address::generate(&env);
    emit_milestone_refunded_event(&env, 1, 0, 1_000, &recipient);
    assert!(!env.events().all().is_empty());
}

#[test]
fn milestone_refunded_accepts_zero_amount() {
    let env = env();
    let recipient = soroban_sdk::Address::generate(&env);
    emit_milestone_refunded_event(&env, 1, 0, 0, &recipient);
    assert!(!env.events().all().is_empty());
}

#[test]
fn milestone_refunded_accepts_max_milestone_index() {
    let env = env();
    let recipient = soroban_sdk::Address::generate(&env);
    emit_milestone_refunded_event(&env, 1, MAX_MILESTONES - 1, 500, &recipient);
    assert!(!env.events().all().is_empty());
}

#[test]
#[should_panic]
fn milestone_refunded_rejects_zero_contract_id() {
    let env = env();
    let recipient = soroban_sdk::Address::generate(&env);
    emit_milestone_refunded_event(&env, 0, 0, 1_000, &recipient);
}

#[test]
#[should_panic]
fn milestone_refunded_rejects_out_of_bounds_index() {
    let env = env();
    let recipient = soroban_sdk::Address::generate(&env);
    emit_milestone_refunded_event(&env, 1, MAX_MILESTONES, 1_000, &recipient);
}

#[test]
#[should_panic]
fn milestone_refunded_rejects_negative_amount() {
    let env = env();
    let recipient = soroban_sdk::Address::generate(&env);
    emit_milestone_refunded_event(&env, 1, 0, -1, &recipient);
}

// ── emit_milestone_approved_event ────────────────────────────────────────────

#[test]
fn milestone_approved_accepts_valid() {
    let env = env();
    let approver = soroban_sdk::Address::generate(&env);
    emit_milestone_approved_event(&env, 1, 0, &approver);
    assert!(!env.events().all().is_empty());
}

#[test]
fn milestone_approved_accepts_max_milestone_index() {
    let env = env();
    let approver = soroban_sdk::Address::generate(&env);
    emit_milestone_approved_event(&env, 1, MAX_MILESTONES - 1, &approver);
    assert!(!env.events().all().is_empty());
}

#[test]
fn milestone_approved_accepts_id_boundary() {
    let env = env();
    let approver = soroban_sdk::Address::generate(&env);
    emit_milestone_approved_event(&env, u32::MAX, 0, &approver);
    assert!(!env.events().all().is_empty());
}

#[test]
#[should_panic]
fn milestone_approved_rejects_zero_contract_id() {
    let env = env();
    let approver = soroban_sdk::Address::generate(&env);
    emit_milestone_approved_event(&env, 0, 0, &approver);
}

#[test]
#[should_panic]
fn milestone_approved_rejects_out_of_bounds_index() {
    let env = env();
    let approver = soroban_sdk::Address::generate(&env);
    emit_milestone_approved_event(&env, 1, MAX_MILESTONES, &approver);
}

#[test]
#[should_panic]
fn milestone_approved_rejects_index_far_out_of_bounds() {
    let env = env();
    let approver = soroban_sdk::Address::generate(&env);
    emit_milestone_approved_event(&env, 1, u32::MAX, &approver);
}

// ── emit_work_evidence_submitted_event ───────────────────────────────────────

#[test]
fn work_evidence_accepts_valid() {
    let env = env();
    let submitter = soroban_sdk::Address::generate(&env);
    let evidence = String::from_str(&env, "ipfs://bafybeiexample");
    emit_work_evidence_submitted_event(&env, 1, 0, &submitter, &evidence);
    assert!(!env.events().all().is_empty());
}

#[test]
fn work_evidence_accepts_single_byte() {
    let env = env();
    let submitter = soroban_sdk::Address::generate(&env);
    let evidence = String::from_str(&env, "x");
    emit_work_evidence_submitted_event(&env, 1, 0, &submitter, &evidence);
    assert!(!env.events().all().is_empty());
}

#[test]
fn work_evidence_accepts_exactly_max_bytes() {
    let env = env();
    let submitter = soroban_sdk::Address::generate(&env);
    // Build a string of exactly MAX_WORK_EVIDENCE_BYTES ASCII characters
    let evidence = String::from_str(&env, &"a".repeat(MAX_WORK_EVIDENCE_BYTES as usize));
    assert_eq!(evidence.len(), MAX_WORK_EVIDENCE_BYTES);
    emit_work_evidence_submitted_event(&env, 1, 0, &submitter, &evidence);
    assert!(!env.events().all().is_empty());
}

#[test]
fn work_evidence_accepts_max_milestone_index() {
    let env = env();
    let submitter = soroban_sdk::Address::generate(&env);
    let evidence = String::from_str(&env, "ipfs://valid");
    emit_work_evidence_submitted_event(&env, 1, MAX_MILESTONES - 1, &submitter, &evidence);
    assert!(!env.events().all().is_empty());
}

#[test]
#[should_panic]
fn work_evidence_rejects_zero_contract_id() {
    let env = env();
    let submitter = soroban_sdk::Address::generate(&env);
    let evidence = String::from_str(&env, "ipfs://valid");
    emit_work_evidence_submitted_event(&env, 0, 0, &submitter, &evidence);
}

#[test]
#[should_panic]
fn work_evidence_rejects_out_of_bounds_index() {
    let env = env();
    let submitter = soroban_sdk::Address::generate(&env);
    let evidence = String::from_str(&env, "ipfs://valid");
    emit_work_evidence_submitted_event(&env, 1, MAX_MILESTONES, &submitter, &evidence);
}

#[test]
#[should_panic]
fn work_evidence_rejects_empty_string() {
    let env = env();
    let submitter = soroban_sdk::Address::generate(&env);
    let evidence = String::from_str(&env, "");
    emit_work_evidence_submitted_event(&env, 1, 0, &submitter, &evidence);
}

#[test]
#[should_panic]
fn work_evidence_rejects_one_byte_over_max() {
    let env = env();
    let submitter = soroban_sdk::Address::generate(&env);
    // MAX_WORK_EVIDENCE_BYTES + 1 bytes
    let evidence = String::from_str(&env, &"a".repeat(MAX_WORK_EVIDENCE_BYTES as usize + 1));
    assert_eq!(evidence.len(), MAX_WORK_EVIDENCE_BYTES + 1);
    emit_work_evidence_submitted_event(&env, 1, 0, &submitter, &evidence);
}

// ── Cross-function: milestone-index boundary is exactly MAX_MILESTONES - 1 ───

#[test]
fn all_milestone_functions_accept_last_valid_index() {
    let env = env();
    let addr = soroban_sdk::Address::generate(&env);
    let last_valid = MAX_MILESTONES - 1;

    emit_milestone_released_event(&env, 1, last_valid, 500, 500, 0, &addr);
    emit_milestone_refunded_event(&env, 1, last_valid, 500, &addr);
    emit_milestone_approved_event(&env, 1, last_valid, &addr);
    let evidence = String::from_str(&env, "ok");
    emit_work_evidence_submitted_event(&env, 1, last_valid, &addr, &evidence);

    // At least 4 events emitted
    assert!(env.events().all().len() >= 4);
}

#[test]
#[should_panic]
fn all_milestone_functions_reject_at_exact_max_index_released() {
    let env = env();
    let addr = soroban_sdk::Address::generate(&env);
    emit_milestone_released_event(&env, 1, MAX_MILESTONES, 500, 500, 0, &addr);
}

#[test]
#[should_panic]
fn all_milestone_functions_reject_at_exact_max_index_refunded() {
    let env = env();
    let addr = soroban_sdk::Address::generate(&env);
    emit_milestone_refunded_event(&env, 1, MAX_MILESTONES, 500, &addr);
}

#[test]
#[should_panic]
fn all_milestone_functions_reject_at_exact_max_index_approved() {
    let env = env();
    let addr = soroban_sdk::Address::generate(&env);
    emit_milestone_approved_event(&env, 1, MAX_MILESTONES, &addr);
}

#[test]
#[should_panic]
fn all_milestone_functions_reject_at_exact_max_index_evidence() {
    let env = env();
    let addr = soroban_sdk::Address::generate(&env);
    let evidence = String::from_str(&env, "ok");
    emit_work_evidence_submitted_event(&env, 1, MAX_MILESTONES, &addr, &evidence);
}
