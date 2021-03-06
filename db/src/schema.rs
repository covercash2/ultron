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

table! {
    inventory (server_id, user_id, item_id) {
        server_id -> Text,
        user_id -> Text,
        item_id -> Integer,
    }
}

table! {
    items (id) {
        id -> Integer,
        name -> Text,
        description -> Text,
        emoji -> Text,
        price -> Integer,
        available -> Integer,
    }
}

joinable!(inventory -> items (item_id));

allow_tables_to_appear_in_same_query!(
    bank_accounts,
    channel_users,
    inventory,
    items,
);
