use std::sync::Arc;

use serenity::futures::lock::Mutex;
use tokio::sync::mpsc::{Receiver, Sender};

use chrono::{DateTime, Utc};

use crate::error::{Error, Result};

use super::{ChannelId, UserId, Receipt};

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
	    receive_channel
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
#[derive(Debug)]
pub enum Transaction {
    /// Transfer coins from one user to another
    Transfer {
        channel_id: ChannelId,
        from_user: UserId,
        to_user: UserId,
        amount: i64,
    },
    /// Dump the account data
    GetAllBalances(ChannelId),
    GetUserBalance {
        channel_id: ChannelId,
        user_id: UserId,
    },
    Tip {
        channel_id: ChannelId,
        from_user: UserId,
        to_user: UserId,
    },
    Untip {
        channel_id: ChannelId,
        from_user: UserId,
        to_user: UserId,
    },
    /// Give some coins to a user once per day
    Daily {
        channel_id: ChannelId,
        user_id: UserId,
        timestamp: DateTime<Utc>,
    },
}

#[derive(Debug)]
pub enum TransactionStatus {
    Complete,
    BadDailyRequest { next_epoch: DateTime<Utc> },
    SelfTip,
}