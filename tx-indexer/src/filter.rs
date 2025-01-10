use oura::filters::selection::{Config, Predicate};
use plutus_ledger_api::{
    csl::{lib as csl, pla_to_csl::TryFromPLA},
    v3::{script::MintingPolicyHash, value::CurrencySymbol},
};

/// Interesting transaction components to look for when filtering transactions
/// relevant to the protocol.
/// Set curr_symbols to empty vectors to handle any transaction event indiscriminately.
pub struct Filter {
    pub curr_symbols: Vec<CurrencySymbol>,
}

// We only obtain Transaction events that contain the policy in the output
// NOTE: Must enable 'include_transaction_details' in oura Mapper config.
impl Filter {
    pub fn to_selection_config(self) -> Config {
        Config {
            check: Predicate::AnyOf(vec![
                Predicate::VariantIn(vec!["RollBack".to_string(), "Block".to_string()]),
                Predicate::AllOf(vec![
                    Predicate::VariantIn(vec!["Transaction".to_string()]),
                    if self.curr_symbols.is_empty() {
                        ALWAYS_TRUE
                    } else {
                        Predicate::AnyOf(
                            self.curr_symbols
                                .into_iter()
                                .map(serialize_cur_sym)
                                .map(Predicate::PolicyEquals)
                                .collect(),
                        )
                    },
                ]),
            ]),
        }
    }
}

// Filter predicate that always succeeds.
const ALWAYS_TRUE: Predicate = Predicate::AllOf(vec![]);

fn serialize_cur_sym(cur_sym: CurrencySymbol) -> String {
    match cur_sym {
        CurrencySymbol::Ada => String::new(),
        CurrencySymbol::NativeToken(MintingPolicyHash(script_hash)) => {
            csl::ScriptHash::try_from_pla(&script_hash)
                .unwrap()
                .to_hex()
        }
    }
}
