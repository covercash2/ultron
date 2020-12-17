table! {
    channel_users (server_id, channel_id, user_id) {
        server_id -> Integer,
        channel_id -> Integer,
        user_id -> Integer,
    }
}

table! {
    coin_accounts (server_id, user_id) {
        server_id -> Integer,
        user_id -> Integer,
        balance -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(
    channel_users,
    coin_accounts,
);
