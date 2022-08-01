use std::{
    collections::{btree_map::Entry, BTreeMap, HashMap, HashSet},
    io::{Read, Write},
};

use anyhow::Result;
use csv::{Reader, ReaderBuilder, Writer, WriterBuilder};
use serde::{
    de::{self, value::MapDeserializer},
    Deserialize, Serialize,
};

mod decimal;
mod op_impls;
mod serde_impls;
pub use decimal::Balance;

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Hash)]
#[serde(transparent)]
pub struct ClientId(u16);

#[derive(Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct TransactionId(u32);

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Action {
    Deposit {
        client: ClientId,
        #[serde(rename = "tx")]
        transaction: TransactionId,
        amount: Balance,
    },
    Withdrawal {
        client: ClientId,
        #[serde(rename = "tx")]
        transaction: TransactionId,
        amount: Balance,
    },
    Dispute {
        client: ClientId,
        #[serde(rename = "tx")]
        transaction: TransactionId,
    },
    Resolve {
        client: ClientId,
        #[serde(rename = "tx")]
        transaction: TransactionId,
    },
    Chargeback {
        client: ClientId,
        #[serde(rename = "tx")]
        transaction: TransactionId,
    },
}

pub enum Transaction {
    Deposit {
        client: ClientId,
        transaction: TransactionId,
        amount: Balance,
    },
    Withdrawal {
        client: ClientId,
        transaction: TransactionId,
        amount: Balance,
    },
}

#[derive(Serialize)]
pub struct AccountSummary {
    client: ClientId,
    locked: bool,
    available: Balance,
    held: Balance,
    total: Balance,
}

pub enum TransactionKind {
    Deposit(Balance),
    Withdrawal(Balance),
}

#[derive(Default)]
struct AccountState {
    transaction_amounts: BTreeMap<TransactionId, TransactionKind>,
    disputes: HashSet<TransactionId>,
    locked: bool,
    available: Balance,
    held: Balance,
}

impl AccountStates {
    pub fn summary(&self) -> Vec<AccountSummary> {
        self.accounts
            .iter()
            .map(
                |(
                    &client,
                    &AccountState {
                        locked,
                        ref available,
                        ref held,
                        ..
                    },
                )| {
                    AccountSummary {
                        client,
                        locked,
                        available: available.clone(),
                        held: held.clone(),
                        total: available + held,
                    }
                },
            )
            .collect()
    }

    /// Apply an action against the client
    ///
    /// *Details*:
    /// When a dispute is resolved, subsequent dispute filed will be ignored.
    /// When a dispute is filed against a `Withdrawal` transaction,
    /// some funds will be allocated to the `held` state,
    /// and the reversal will move this portion of funds from `held` to `available`.
    pub fn process(&mut self, action: Action) {
        match action {
            Action::Deposit {
                client,
                transaction,
                amount,
            } => {
                let client = self.accounts.entry(client).or_default();
                if client.locked {
                    return;
                }
                if let Entry::Vacant(e) = client.transaction_amounts.entry(transaction) {
                    e.insert(TransactionKind::Deposit(amount.clone()));
                    client.available += amount;
                }
            }
            Action::Withdrawal {
                client,
                transaction,
                amount,
            } => {
                let client = self.accounts.entry(client).or_default();
                if client.locked {
                    return;
                }
                if let Entry::Vacant(e) = client.transaction_amounts.entry(transaction) {
                    if let Some(available) = client.available.clone() - amount.clone() {
                        client.available = available;
                        e.insert(TransactionKind::Withdrawal(amount));
                    }
                }
            }
            Action::Dispute {
                client,
                transaction,
            } => {
                let client = self.accounts.entry(client).or_default();
                if client.locked {
                    return;
                }
                if client.disputes.contains(&transaction) {
                    return;
                }
                match client.transaction_amounts.get(&transaction) {
                    Some(TransactionKind::Deposit(amount)) => {
                        if let Some(available) = client.available.clone() - amount.clone() {
                            client.available = available;
                            client.held += amount.clone();
                            client.disputes.insert(transaction);
                        }
                    }
                    Some(TransactionKind::Withdrawal(amount)) => {
                        client.held += amount;
                        client.disputes.insert(transaction);
                    }
                    None => {}
                }
            }
            Action::Resolve {
                client,
                transaction,
            } => {
                let client = self.accounts.entry(client).or_default();
                if client.locked {
                    return;
                }
                if !client.disputes.contains(&transaction) {
                    return;
                }
                match client.transaction_amounts.get(&transaction) {
                    Some(TransactionKind::Deposit(amount)) => {
                        if let Some(held) = client.held.clone() - amount.clone() {
                            client.held = held;
                            client.available += amount.clone();
                            client.transaction_amounts.remove(&transaction);
                            client.disputes.remove(&transaction);
                        } else {
                            unreachable!(
                                "the held amount should always be sufficient for dispute resolution"
                            )
                        }
                    }
                    Some(TransactionKind::Withdrawal(amount)) => {
                        if let Some(held) = client.held.clone() - amount.clone() {
                            client.held = held;
                            client.transaction_amounts.remove(&transaction);
                            client.disputes.remove(&transaction);
                        } else {
                            unreachable!(
                                "the held amount should always be sufficient for dispute resolution"
                            )
                        }
                    }
                    None => {}
                }
            }
            Action::Chargeback {
                client,
                transaction,
            } => {
                let client = self.accounts.entry(client).or_default();
                if client.locked {
                    return;
                }
                if !client.disputes.contains(&transaction) {
                    return;
                }
                match client.transaction_amounts.get(&transaction) {
                    Some(TransactionKind::Deposit(amount)) => {
                        if let Some(held) = client.held.clone() - amount.clone() {
                            client.held = held;
                            client.disputes.remove(&transaction);
                            client.locked = true;
                        } else {
                            unreachable!(
                                "the held amount should always be sufficient for dispute resolution"
                            )
                        }
                    }
                    Some(TransactionKind::Withdrawal(amount)) => {
                        if let Some(held) = client.held.clone() - amount.clone() {
                            client.held = held;
                            client.available += amount.clone();
                            client.disputes.remove(&transaction);
                            client.locked = true;
                        } else {
                            unreachable!(
                                "the held amount should always be sufficient for dispute resolution"
                            )
                        }
                    }
                    None => {}
                }
            }
        }
    }
}

#[derive(Default)]
pub struct AccountStates {
    accounts: BTreeMap<ClientId, AccountState>,
}

pub fn aggregate(stream: impl IntoIterator<Item = Action>) -> Vec<AccountSummary> {
    let mut states = AccountStates::default();
    for action in stream {
        states.process(action)
    }
    states.summary()
}

pub fn summaries_from_csv<R: Read>(mut reader: Reader<R>) -> Result<Vec<AccountSummary>> {
    let mut states = AccountStates::default();
    for record in reader.deserialize() {
        let record: HashMap<String, String> = record?;
        states.process(<_>::deserialize(
            MapDeserializer::<_, de::value::Error>::new(
                record.into_iter().map(|(k, v)| (k.trim().to_owned(), v)),
            ),
        )?)
    }
    Ok(states.summary())
}

/// Compute account summary from IO CSV source
pub fn summaries_from_io_csv(reader: impl Read) -> Result<Vec<AccountSummary>> {
    summaries_from_csv(ReaderBuilder::new().from_reader(reader))
}

pub fn write_summary_csv<'a, W: Write>(
    summaries: impl IntoIterator<Item = &'a AccountSummary>,
    mut writer: Writer<W>,
) -> Result<()> {
    for record in summaries {
        writer.serialize(record)?
    }
    Ok(())
}

pub fn write_summary_io_csv<'a>(
    summaries: impl IntoIterator<Item = &'a AccountSummary>,
    writer: impl Write,
) -> Result<()> {
    write_summary_csv(summaries, WriterBuilder::new().from_writer(writer))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn serde_works() {
        serde_json::from_str::<Action>(
            r#"{
            "type": "deposit",
            "client": 1,
            "tx": 1,
            "amount": "1.0"
        }"#,
        )
        .unwrap();
    }

    const TRANSACTION_CSV: &'static str = r#"type, client, tx, amount
deposit, 1,   1, 1.0
deposit, 2,2,2.0
deposit, 1, 3, 2
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0
"#;
    #[test]
    fn csv_works() {
        let mut rdr = ReaderBuilder::new().from_reader(TRANSACTION_CSV.as_bytes());
        let mut records = vec![];
        for record in rdr.deserialize() {
            let record: HashMap<String, String> = record.unwrap();
            records.push(
                Action::deserialize(MapDeserializer::<_, serde::de::value::Error>::new(
                    record.into_iter().map(|(k, v)| (k.trim().to_owned(), v)),
                ))
                .unwrap(),
            );
        }
    }

    #[test]
    fn process_correctly() {
        let summaries =
            summaries_from_csv(ReaderBuilder::new().from_reader(TRANSACTION_CSV.as_bytes()))
                .unwrap();
        let mut output = vec![];
        write_summary_io_csv(&summaries, &mut output).unwrap();
        assert_eq!(
            output,
            r#"client,locked,available,held,total
1,false,1.5000,0.0000,1.5000
2,false,2.0000,0.0000,2.0000
"#
            .as_bytes()
        )
    }

    const TRANSACTION_DISPUTE_CSV: &'static str = r#"type, client, tx, amount
deposit, 1, 1, 1.0
dispute, 1, 1,
chargeback, 1, 1,
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0
chargeback, 2, 2,
dispute, 2, 2,
resolve, 2, 2,
dispute, 2, 2,
withdrawal, 2, 6, 2
"#;
    #[test]
    fn handle_dispute_correctly() {
        let summaries = summaries_from_csv(
            ReaderBuilder::new().from_reader(TRANSACTION_DISPUTE_CSV.as_bytes()),
        )
        .unwrap();
        let mut output = vec![];
        write_summary_io_csv(&summaries, &mut output).unwrap();
        assert_eq!(
            output,
            r#"client,locked,available,held,total
1,true,0.0000,0.0000,0.0000
2,false,0.0000,0.0000,0.0000
"#
            .as_bytes()
        )
    }
}
