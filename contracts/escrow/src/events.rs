use crate::milestones_consts::MAX_MILESTONES;
use crate::milestones_consts::MAX_WORK_EVIDENCE_BYTES;
use crate::types::Contract;
use crate::EscrowError;
use soroban_sdk::{symbol_short, Address, Env};

#[soroban_sdk::contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventInput {
    pub topic: soroban_sdk::Symbol,
    pub contract_id: u32,
    pub data: soroban_sdk::Symbol,
}

/// Maximum number of events processed in a batch operations.
pub const MAX_EVENT_BATCH_SIZE: usize = 100;

// ── Shared validation helpers ─────────────────────────────────────────────────

/// Validate that `contract_id` is non-zero.
///
/// Soroban allocates contract IDs starting from `1`; `0` is never a valid
/// live contract and indicates a caller error or uninitialized field.
///
/// # Panics
/// `InvalidContractId` when `contract_id == 0`.
#[inline]
fn require_valid_contract_id(env: &Env, contract_id: u32) {
    if contract_id == 0 {
        env.panic_with_error(EscrowError::InvalidContractId);
    }
}

/// Validate that `milestone_index` is within the protocol-wide milestone limit.
///
/// This is a defense-in-depth guard: callers such as the release and evidence
/// paths already check the milestone vector length, but emitting an event with
/// an out-of-bounds index would produce misleading off-chain records.
///
/// # Panics
/// `IndexOutOfBounds` when `milestone_index >= MAX_MILESTONES`.
#[inline]
fn require_valid_milestone_index(env: &Env, milestone_index: u32) {
    if milestone_index >= MAX_MILESTONES {
        env.panic_with_error(EscrowError::IndexOutOfBounds);
    }
}

/// Validate that a work-evidence string is non-empty and within the byte-length
/// limit defined by [`MAX_WORK_EVIDENCE_BYTES`].
///
/// # Panics
/// - `EmptyEvidence` when `evidence.len() == 0`.
/// - `EvidenceTooLong` when `evidence.len() > MAX_WORK_EVIDENCE_BYTES`.
#[inline]
fn require_valid_evidence(env: &Env, evidence: &soroban_sdk::String) {
    if evidence.len() == 0 {
        env.panic_with_error(EscrowError::EmptyEvidence);
    }
    if evidence.len() > MAX_WORK_EVIDENCE_BYTES {
        env.panic_with_error(EscrowError::EvidenceTooLong);
    }
}

// ── Public event-emission functions ──────────────────────────────────────────

/// Emits an indexed event on contract state changes to assist off-chain indexers
/// in cheaply reconstructing contract lifecycle history and financial balances.
///
/// # Event Specification
/// - **Topic**: `(symbol_short!("contract"), contract_id: u32)`
/// - **Payload**: `(status: u32, funded_amount: i128, released_amount: i128, refunded_amount: i128, total_deposited: i128)`
///
/// # Panics
/// - `InvalidContractId` if `contract_id` is zero.
/// - `AmountMustBePositive` if any amount field is negative.
pub fn emit_contract_indexed_event(env: &Env, contract_id: u32, contract: &Contract) {
    require_valid_contract_id(env, contract_id);

    validate_event_amounts(
        contract.funded_amount,
        contract.released_amount,
        contract.refunded_amount,
        contract.total_deposited,
    )
    .unwrap_or_else(|e| env.panic_with_error(e));

    env.events().publish(
        (symbol_short!("contract"), contract_id),
        (
            contract.status as u32,
            contract.funded_amount,
            contract.released_amount,
            contract.refunded_amount,
            contract.total_deposited,
        ),
    );
}

/// Validate that event payload amounts are non-negative.
/// Returns `Ok(())` when all amounts are >= 0.
pub(crate) fn validate_event_amounts(
    funded_amount: i128,
    released_amount: i128,
    refunded_amount: i128,
    total_deposited: i128,
) -> Result<(), crate::EscrowError> {
    if funded_amount < 0 || released_amount < 0 || refunded_amount < 0 || total_deposited < 0 {
        return Err(EscrowError::AmountMustBePositive);
    }
    Ok(())
}

/// Emits an indexed event when a dispute is opened on a contract.
///
/// # Event Specification
/// - **Topic**: `(symbol_short!("dispute"), symbol_short!("opened"))`
/// - **Payload**: `(contract_id: u32, caller: Address, funded_amount: i128, released_amount: i128, refunded_amount: i128)`
///
/// # Panics
/// - `InvalidContractId` if `contract_id` is zero.
/// - `AmountMustBePositive` if any of `funded_amount`, `released_amount`, or
///   `refunded_amount` on the contract snapshot is negative.
pub fn emit_dispute_opened_event(
    env: &Env,
    contract_id: u32,
    caller: &Address,
    contract: &Contract,
) {
    require_valid_contract_id(env, contract_id);

    validate_event_amounts(
        contract.funded_amount,
        contract.released_amount,
        contract.refunded_amount,
        contract.total_deposited,
    )
    .unwrap_or_else(|e| env.panic_with_error(e));

    env.events().publish(
        (symbol_short!("dispute"), symbol_short!("opened")),
        (
            contract_id,
            caller.clone(),
            contract.funded_amount,
            contract.released_amount,
            contract.refunded_amount,
        ),
    );
}

/// Emits an indexed event when a dispute is resolved.
///
/// # Event Specification
/// - **Topic**: `(symbol_short!("dispute"), symbol_short!("resolved"))`
/// - **Payload**: `(contract_id: u32, client_payout: i128, freelancer_payout: i128, resolution_code: u32, final_status: u32)`
///
/// # Panics
/// - `InvalidContractId` if `contract_id` is zero.
/// - `AmountMustBePositive` if `client_payout` or `freelancer_payout` is negative.
pub fn emit_dispute_resolved_event(
    env: &Env,
    contract_id: u32,
    client_payout: i128,
    freelancer_payout: i128,
    resolution_code: u32,
    final_status: crate::types::ContractStatus,
) {
    require_valid_contract_id(env, contract_id);

    if client_payout < 0 || freelancer_payout < 0 {
        env.panic_with_error(EscrowError::AmountMustBePositive);
    }

    env.events().publish(
        (symbol_short!("dispute"), symbol_short!("resolved")),
        (
            contract_id,
            client_payout,
            freelancer_payout,
            resolution_code,
            final_status as u32,
        ),
    );
}

/// Emits an event when a milestone is released to a freelancer.
///
/// # Panics
/// - `InvalidContractId` if `contract_id` is zero.
/// - `IndexOutOfBounds` if `milestone_index >= MAX_MILESTONES`.
/// - `AmountMustBePositive` if `amount`, `gross_amount`, or `fee` is negative.
pub fn emit_milestone_released_event(
    env: &Env,
    contract_id: u32,
    milestone_index: u32,
    amount: i128,
    gross_amount: i128,
    fee: i128,
    recipient: &Address,
) {
    require_valid_contract_id(env, contract_id);
    require_valid_milestone_index(env, milestone_index);

    if amount < 0 || gross_amount < 0 || fee < 0 {
        env.panic_with_error(EscrowError::AmountMustBePositive);
    }

    env.events().publish(
        (symbol_short!("milestone"), symbol_short!("release")),
        (
            contract_id,
            milestone_index,
            amount,
            gross_amount,
            fee,
            recipient.clone(),
            env.ledger().timestamp(),
        ),
    );
}

/// Emits an event when a milestone is refunded to the client.
///
/// # Panics
/// - `InvalidContractId` if `contract_id` is zero.
/// - `IndexOutOfBounds` if `milestone_index >= MAX_MILESTONES`.
/// - `AmountMustBePositive` if `amount` is negative.
pub fn emit_milestone_refunded_event(
    env: &Env,
    contract_id: u32,
    milestone_index: u32,
    amount: i128,
    recipient: &Address,
) {
    require_valid_contract_id(env, contract_id);
    require_valid_milestone_index(env, milestone_index);

    if amount < 0 {
        env.panic_with_error(EscrowError::AmountMustBePositive);
    }

    env.events().publish(
        (symbol_short!("milestone"), symbol_short!("refund")),
        (
            contract_id,
            milestone_index,
            amount,
            recipient.clone(),
            env.ledger().timestamp(),
        ),
    );
}

/// Emits an event when a milestone is approved by client or arbiter.
///
/// # Panics
/// - `InvalidContractId` if `contract_id` is zero.
/// - `IndexOutOfBounds` if `milestone_index >= MAX_MILESTONES`.
pub fn emit_milestone_approved_event(
    env: &Env,
    contract_id: u32,
    milestone_index: u32,
    approver: &Address,
) {
    require_valid_contract_id(env, contract_id);
    require_valid_milestone_index(env, milestone_index);

    env.events().publish(
        (symbol_short!("milestone"), symbol_short!("approved")),
        (
            contract_id,
            milestone_index,
            approver.clone(),
            env.ledger().timestamp(),
        ),
    );
}

/// Emits an event when work evidence is submitted for a milestone.
///
/// # Panics
/// - `InvalidContractId` if `contract_id` is zero.
/// - `IndexOutOfBounds` if `milestone_index >= MAX_MILESTONES`.
/// - `EmptyEvidence` if `evidence` is empty.
/// - `EvidenceTooLong` if `evidence.len() > MAX_WORK_EVIDENCE_BYTES`.
pub fn emit_work_evidence_submitted_event(
    env: &Env,
    contract_id: u32,
    milestone_index: u32,
    submitter: &Address,
    evidence: &soroban_sdk::String,
) {
    require_valid_contract_id(env, contract_id);
    require_valid_milestone_index(env, milestone_index);
    require_valid_evidence(env, evidence);

    env.events().publish(
        (symbol_short!("milestone"), symbol_short!("evidence")),
        (
            contract_id,
            milestone_index,
            submitter.clone(),
            evidence.clone(),
            env.ledger().timestamp(),
        ),
    );
}
