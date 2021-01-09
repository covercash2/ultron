use std::sync::Arc;

use serenity::futures::lock::Mutex;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::error::{Error, Result};

use super::{ChannelId, Receipt, ServerId, UserId};

pub struct TransactionSender {
    send_channel: Sender<Transaction>,
    // receivers aren't thread safe, so we need some boxes here
    receive_channel: Arc<Mutex<Receiver<Receipt>>>,
}

impl TransactionSender {
    pub fn new(send_channel: Sender<Transaction>, receive_channel: Receiver<Receipt>) -> Self {
        let receive_channel = Arc::new(Mutex::new(receive_channel));
        TransactionSender {
            send_channel,
            receive_channel,
        }
    }

    /// Send a transaction to the bank thread.
    /// Returns output to say in chat.
    pub async fn send_transaction(&self, transaction: Transaction) -> Result<Receipt> {
        let mut sender = self.send_channel.clone();
        sender.send(transaction).await?;
        let mut lock = self.receive_channel.lock().await;
        if let Some(receipt) = lock.recv().await {
            Ok(receipt)
        } else {
            Err(Error::TransactionReceipt)
        }
    }
}

/// Interactions with the Bank are handled through transactions.
/// These transactions are sent over channels in the [`bank_loop`]
/// to be processed by [`Bank::process_transaction`].
#[derive(Debug, Clone)]
pub struct Transaction {
    pub from_user: UserId,
    pub server_id: ServerId,
    pub channel_id: ChannelId,
    pub operation: Operation,
}

#[derive(Debug, Clone)]
pub enum Operation {
    Transfer {
        to_user: UserId,
        amount: i64,
    },
    GetUserBalance,
    /// Dump items table
    GetAllItems,
}

#[derive(Debug, Clone)]
pub enum TransactionStatus {
    Complete,
}
