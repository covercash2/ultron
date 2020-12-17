table! {
    bank_accounts (server_id, user_id) {
        server_id -> Text,
        user_id -> Text,
        balance -> Integer,
    }
}

table! {
    channel_users (server_id, channel_id, user_id) {
        server_id -> Text,
        channel_id -> Text,
        user_id -> Text,
    }
}

allow_tables_to_appear_in_same_query!(
    bank_accounts,
    channel_users,
);
