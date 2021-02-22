-- Your SQL goes here
create table bank_accounts (
       server_id text not null,
       user_id text not null,
       balance integer not null,
       primary key (server_id, user_id)
);

create table channel_users (
       server_id text not null,
       channel_id text not null,
       user_id text not null,
       primary key (server_id, channel_id, user_id)
);
