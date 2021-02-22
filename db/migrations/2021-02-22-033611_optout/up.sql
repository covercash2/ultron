-- Your SQL goes here
create table optout (
       server_id Text not null,
       user_id Text not null,
       primary key (server_id, user_id)
);
