//! Transaction Info builder

use num_bigint::BigInt;
use plutus_ledger_api::v3::{
    address::Credential,
    assoc_map::AssocMap,
    crypto::{LedgerBytes, PaymentPubKeyHash},
    datum::{Datum, DatumHash},
    interval::Interval,
    redeemer::Redeemer,
    transaction::{
        GovernanceActionId, POSIXTimeRange, ProposalProcedure, ScriptPurpose, TransactionHash,
        TransactionInfo, TransactionInput, TransactionOutput, TxCert, TxInInfo, Vote, Voter,
    },
    value::{AssetClass, Lovelace, Value},
};
use std::collections::{BTreeMap, BTreeSet};

/// Simple TransactionInfo builder
pub struct TxScaffold {
    inputs: BTreeMap<TransactionInput, TxScaffoldInput>,
    reference_inputs: BTreeMap<TransactionInput, TransactionOutput>,
    outputs: Vec<TransactionOutput>,
    mint: Vec<(AssetClass, i64, Redeemer)>,
    tx_certs: Vec<TxCert>,
    withdrawals: BTreeMap<Credential, u64>,
    valid_range: POSIXTimeRange,
    signatories: BTreeSet<PaymentPubKeyHash>,
    votes: BTreeMap<Voter, BTreeMap<GovernanceActionId, Vote>>,
    proposal_procedures: Vec<ProposalProcedure>,
    current_treasury_amount: Option<BigInt>,
    treasury_donation: Option<BigInt>,
}

/// Input to a transaction
pub enum TxScaffoldInput {
    /// Input from a public key wallet
    PubKeyInput { output: TransactionOutput },
    /// Input from a validator address with the attached datum and redeemer
    ScriptInput {
        output: TransactionOutput,
        datum: Option<DatumFromWitness>,
        redeemer: Redeemer,
    },
}

/// Datum and its hash
pub type DatumFromWitness = (DatumHash, Datum);

impl TxScaffoldInput {
    fn output(&self) -> TransactionOutput {
        match self {
            TxScaffoldInput::PubKeyInput { output } => output.clone(),
            TxScaffoldInput::ScriptInput { output, .. } => output.clone(),
        }
    }
}

impl Default for TxScaffold {
    fn default() -> Self {
        TxScaffold {
            inputs: BTreeMap::new(),
            reference_inputs: BTreeMap::new(),
            outputs: Vec::new(),
            mint: Vec::new(),
            tx_certs: Vec::new(),
            withdrawals: BTreeMap::new(),
            valid_range: Interval::Always.into(),
            signatories: BTreeSet::new(),
            votes: BTreeMap::new(),
            proposal_procedures: Vec::new(),
            current_treasury_amount: None,
            treasury_donation: None,
        }
    }
}

impl TxScaffold {
    /// Start an empty scaffold
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a transaction input
    pub fn add_input(mut self, reference: TransactionInput, input: TxScaffoldInput) -> Self {
        self.inputs.insert(reference, input);
        self
    }

    /// Add an inpup from a public key wallet
    pub fn add_pub_key_input(
        mut self,
        reference: TransactionInput,
        output: TransactionOutput,
    ) -> Self {
        self.inputs
            .insert(reference, TxScaffoldInput::PubKeyInput { output });
        self
    }

    /// Add a input from a validator address
    pub fn add_script_input(
        mut self,
        reference: TransactionInput,
        output: TransactionOutput,
        datum: Option<DatumFromWitness>,
        redeemer: Redeemer,
    ) -> Self {
        self.inputs.insert(
            reference,
            TxScaffoldInput::ScriptInput {
                output,
                datum,
                redeemer,
            },
        );
        self
    }

    /// Add multiple transaction inputs
    pub fn add_inputs(mut self, mut inputs: BTreeMap<TransactionInput, TxScaffoldInput>) -> Self {
        self.inputs.append(&mut inputs);
        self
    }

    /// Add a reference input
    pub fn add_reference_input(
        mut self,
        reference: TransactionInput,
        output: TransactionOutput,
    ) -> Self {
        self.reference_inputs.insert(reference, output);
        self
    }

    /// Add multiple reference inputs
    pub fn add_reference_inputs(
        mut self,
        mut inputs: BTreeMap<TransactionInput, TransactionOutput>,
    ) -> Self {
        self.reference_inputs.append(&mut inputs);
        self
    }

    /// Add a transaction output
    /// Output order will be preserved
    pub fn add_output(mut self, output: TransactionOutput) -> Self {
        self.outputs.push(output);
        self
    }

    /// Add multiple transaction outputs
    pub fn add_outputs(mut self, mut outputs: Vec<TransactionOutput>) -> Self {
        self.outputs.append(&mut outputs);
        self
    }

    /// Add new minted tokens with their corresponding redeemer
    pub fn add_mint(mut self, asset_class: AssetClass, amount: i64, redeemer: Redeemer) -> Self {
        self.mint.push((asset_class, amount, redeemer));
        self
    }

    /// Add a Tx Cert
    pub fn add_txcert(mut self, dcert: TxCert) -> Self {
        self.tx_certs.push(dcert);
        self
    }

    /// Add multiple TxCerts
    pub fn add_dcerts(mut self, mut dcerts: Vec<TxCert>) -> Self {
        self.tx_certs.append(&mut dcerts);
        self
    }

    /// Add a withdrawal
    pub fn add_withdrawals(mut self, mut withdrawals: BTreeMap<Credential, u64>) -> Self {
        self.withdrawals.append(&mut withdrawals);
        self
    }

    /// Add multiple withdrawals
    pub fn add_withdrawal(mut self, staking_credential: Credential, amount: u64) -> Self {
        self.withdrawals.insert(staking_credential, amount);
        self
    }

    /// Set the validity range of the transaction
    pub fn set_valid_range(mut self, valid_range: POSIXTimeRange) -> Self {
        self.valid_range = valid_range;
        self
    }

    /// Add a required signer of the transaction
    pub fn add_signatory(mut self, signatory: PaymentPubKeyHash) -> Self {
        self.signatories.insert(signatory);
        self
    }

    /// Add multiple required signers of the transaction
    pub fn add_signatories(mut self, mut signatories: BTreeSet<PaymentPubKeyHash>) -> Self {
        self.signatories.append(&mut signatories);
        self
    }

    pub fn add_votes(
        mut self,
        mut votes: BTreeMap<Voter, BTreeMap<GovernanceActionId, Vote>>,
    ) -> Self {
        self.votes.append(&mut votes);
        self
    }

    pub fn add_proposal_procedures(
        mut self,
        mut proposal_procedures: Vec<ProposalProcedure>,
    ) -> Self {
        self.proposal_procedures.append(&mut proposal_procedures);
        self
    }

    pub fn add_current_treasury_amount(mut self, current_treasury_amount: Option<BigInt>) -> Self {
        self.current_treasury_amount = current_treasury_amount;
        self
    }
    pub fn add_treasury_donation(mut self, treasury_donation: Option<BigInt>) -> Self {
        self.treasury_donation = treasury_donation;
        self
    }

    /// Build a TransactionInfo
    pub fn build(self) -> TransactionInfo {
        TransactionInfo {
            inputs: self
                .inputs
                .iter()
                .map(|(reference, scaffold_in)| {
                    TxInInfo {
                        reference: reference.clone(),
                        output: scaffold_in.output(),
                    }
                    .clone()
                })
                .collect(),
            reference_inputs: self
                .reference_inputs
                .iter()
                .map(|(reference, output)| TxInInfo {
                    reference: reference.clone(),
                    output: output.clone(),
                })
                .collect(),
            outputs: self.outputs,
            fee: Lovelace(BigInt::ZERO),
            mint: self
                .mint
                .iter()
                .fold(Value::new(), |others, (asset_class, amount, _red)| {
                    others
                        + Value::token_value(
                            &asset_class.currency_symbol,
                            &asset_class.token_name,
                            &BigInt::from(*amount),
                        )
                }),
            tx_certs: self.tx_certs,
            wdrl: AssocMap(
                self.withdrawals
                    .into_iter()
                    .map(|(credential, amount)| (credential, Lovelace(BigInt::from(amount))))
                    .collect(),
            ),
            valid_range: self.valid_range,
            signatories: self.signatories.into_iter().collect(),
            redeemers: AssocMap(
                self.inputs
                    .iter()
                    .filter_map(|(reference, scaffold_in)| match scaffold_in {
                        TxScaffoldInput::ScriptInput { redeemer, .. } => {
                            Some((ScriptPurpose::Spending(reference.clone()), redeemer.clone()))
                        }
                        TxScaffoldInput::PubKeyInput { .. } => None,
                    })
                    .chain(self.mint.iter().map(|(asset_class, _amount, red)| {
                        (
                            ScriptPurpose::Minting(asset_class.currency_symbol.clone()),
                            red.clone(),
                        )
                    }))
                    .collect(),
            ),
            datums: AssocMap(
                self.inputs
                    .into_iter()
                    .filter_map(|(_reference, scaffold_in)| match scaffold_in {
                        TxScaffoldInput::ScriptInput { datum, .. } => datum,
                        TxScaffoldInput::PubKeyInput { .. } => None,
                    })
                    .collect(),
            ),
            id: TransactionHash(LedgerBytes(Vec::new())),
            votes: AssocMap(
                self.votes
                    .into_iter()
                    .map(|(voter, votes)| (voter, AssocMap(votes.into_iter().collect())))
                    .collect(),
            ),
            proposal_procedures: self.proposal_procedures,
            current_treasury_amount: self.current_treasury_amount.map(Lovelace),
            treasury_donation: self.treasury_donation.map(Lovelace),
        }
    }
}
