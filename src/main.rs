use std::{fmt::Display, process};

use std::collections::HashMap;
use std::env;
use std::error::Error;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(PartialEq)]
enum TxType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize)]
/// Represents incoming transaction from csv.
struct Transaction {
    client: u16,
    tx: u32,
    amount: Option<f32>,
    r#type: TxType,
}

#[derive(Debug, Serialize)]
struct Account {
    client: u16,
    available: f32,
    held: f32,
    total: f32,
    locked: bool,
}

/// Writes out account data with 4 precision points.
impl Display for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{},{:.4},{:.4},{:.4},{}",
            self.client, self.available, self.held, self.total, self.locked
        )
    }
}
/// Verify matching client id and non locked account for every operation.
impl Account {
    /// Add deposit amount to an Account.
    fn deposit(&mut self, record: &Transaction) -> bool {
        if !self.locked && self.client == record.client {
            self.available += record.amount.unwrap_or(0.0);
            self.total += record.amount.unwrap_or(0.0);
            return true;
        }
        false
    }
    /// Deduct withdrawal amount from an Account.
    /// Ignore withdrawal request from an account with insufficient available funds.
    fn withdrawal(&mut self, record: &Transaction) -> bool {
        if self.available >= record.amount.unwrap_or(0.0)
            && !self.locked
            && self.client == record.client
        {
            self.available -= record.amount.unwrap_or(0.0);
            self.total -= record.amount.unwrap_or(0.0);
            return true;
        }
        false
    }
    /// Held funds from a disputed transaction.
    fn dispute(&mut self, record: &Transaction) -> bool {
        if record.r#type == TxType::Withdrawal
            || record.r#type == TxType::Deposit && !self.locked && self.client == record.client
        {
            self.held += record.amount.unwrap_or(0.0);
            self.available -= record.amount.unwrap_or(0.0);
            return true;
        }
        false
    }
    /// Add resolved amount from a resolved transaction.
    fn resolve(&mut self, record: &Transaction) -> bool {
        if record.r#type == TxType::Dispute && !self.locked && self.client == record.client {
            self.held -= record.amount.unwrap_or(0.0);
            self.available += record.amount.unwrap_or(0.0);
            return true;
        }
        false
    }
    /// Deduct a disputed transaction amount.
    /// Accept only disputed transaction, else ignore request.
    fn chargeback(&mut self, record: &Transaction) -> bool {
        if record.r#type == TxType::Dispute && self.client == record.client {
            self.locked = true;
            self.total -= record.amount.unwrap_or(0.0);
            self.held -= record.amount.unwrap_or(0.0);
            return true;
        }
        false
    }
}

/// Add new client to the ledger , only deposit tnx are valid.
/// otherwise account is open with 0 funds.
fn create_new_account(record: &Transaction) -> Account {
    let total = match record.r#type {
        TxType::Deposit => record.amount.unwrap_or(0.0),
        _ => 0.0,
    };
    Account {
        client: record.client,
        available: total,
        held: 0.0,
        total,
        locked: false,
    }
}
/// Reads csv file prints out ledger final state.
fn process_records(csv: &String) -> Result<HashMap<u16, Account>, Box<dyn Error>> {
    let mut tx_history: HashMap<u32, Transaction> = HashMap::new();
    let mut ledger: HashMap<u16, Account> = HashMap::new();
    let mut rdr = csv::Reader::from_path(csv)?;
    let mut successful = false;
    for result in rdr.deserialize() {
        let mut record: Transaction = result?;
        ledger
            .entry(record.client)
            .and_modify(|account| {
                // fetch the referenced tx data for special tx type and verify the client id.

                let transaction = match record.r#type {
                    TxType::Dispute | TxType::Resolve | TxType::Chargeback => {
                        tx_history.get(&record.tx)
                    }
                    TxType::Withdrawal | TxType::Deposit => Some(&record),
                };

                if let Some(rc) = transaction {
                    // match on incoming tx and use the correct tx data to process.
                    successful = match record.r#type {
                        TxType::Deposit => account.deposit(rc),
                        TxType::Withdrawal => account.withdrawal(rc),
                        TxType::Dispute => account.dispute(rc),
                        TxType::Resolve => account.resolve(rc),
                        TxType::Chargeback => account.chargeback(rc),
                    };
                    // need to update the tnx amount for tnx that is missing amount.
                    record.amount = rc.amount;
                }
            })
            .or_insert_with(|| {
                successful = true;
                create_new_account(&record)
            });
        // only update / insert successful transactions
        if successful {
            tx_history.insert(record.tx, record);
        }
    }
    Ok(ledger)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let file = &args[1];

    match process_records(file) {
        Ok(ledger) => {
            println!("client, available, held, total, locked");
            ledger.values().for_each(|account| println!("{:}", account))
        }
        Err(err) => {
            println!("error processing records : {}", err);
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispute() -> Result<(), Box<dyn Error>> {
        let ledger = process_records(&"src/tests/input/dispute.csv".to_string()).unwrap();
        assert_eq!(ledger[&1].available, -1.0);
        assert_eq!(ledger[&1].held, 11.5);
        assert_eq!(ledger[&1].total, 10.5);
        assert!(!ledger[&1].locked);
        Ok(())
    }

    #[test]
    fn test_chargeback() -> Result<(), Box<dyn Error>> {
        let ledger = process_records(&"src/tests/input/chargeback.csv".to_string()).unwrap();
        assert_eq!(ledger[&2].available, -3.0);
        assert_eq!(ledger[&2].held, 0.0);
        assert_eq!(ledger[&2].total, -3.0);
        assert!(ledger[&2].locked);
        Ok(())
    }
    #[test]
    fn test_resolved() -> Result<(), Box<dyn Error>> {
        let ledger = process_records(&"src/tests/input/resolve.csv".to_string()).unwrap();
        assert_eq!(ledger[&1].available, 0.5);
        assert_eq!(ledger[&1].held, 0.0);
        assert_eq!(ledger[&1].total, 0.5);
        assert!(!ledger[&1].locked);
        Ok(())
    }
    #[test]
    fn test_withdrawal() -> Result<(), Box<dyn Error>> {
        let ledger = process_records(&"src/tests/input/withdrawal.csv".to_string()).unwrap();
        assert_eq!(ledger[&1].available, 10.0);
        assert_eq!(ledger[&1].held, 0.0);
        assert!(!ledger[&1].locked);
        Ok(())
    }
    #[test]
    fn test_mixed() -> Result<(), Box<dyn Error>> {
        let ledger = process_records(&"src/tests/input/mixed.csv".to_string()).unwrap();
        let expect_results = vec![
            Account {
                client: 1,
                available: 199.0,
                held: 0.0,
                total: 199.0,
                locked: true,
            },
            Account {
                client: 2,
                available: 102.0,
                held: 0.0,
                total: 102.0,
                locked: false,
            },
            Account {
                client: 3,
                available: 200.0,
                held: 100.0,
                total: 300.0,
                locked: false,
            },
            Account {
                client: 4,
                available: 221.0,
                held: 0.0,
                total: 221.0,
                locked: false,
            },
            Account {
                client: 5,
                available: 241.0,
                total: 241.0,
                held: 0.0,
                locked: false,
            },
            Account {
                client: 6,
                available: 342.0,
                total: 342.0,
                held: 0.0,
                locked: false,
            },
            Account {
                client: 7,
                available: 134.0,
                total: 134.0,
                held: 0.0,
                locked: false,
            },
        ];
        expect_results.iter().for_each(|ac| {
            assert_eq!(ledger[&ac.client].available, ac.available);
            assert_eq!(ledger[&ac.client].held, ac.held);
            assert_eq!(ledger[&ac.client].total, ac.total);
            assert_eq!(ledger[&ac.client].locked, ac.locked);
        });
        Ok(())
    }
}
