-- Your SQL goes here
drop table inventory;

create table inventory (
       server_id text not null,
       user_id text not null,
       item_id integer not null,
       primary key (server_id, user_id, item_id),
       foreign key (item_id) references items(id)
)
