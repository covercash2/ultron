-- Your SQL goes here
create table items (
       id integer not null primary key,
       name text not null,
       description text not null,
       emoji text not null,
       price integer not null
);

create table inventory (
       user_id text not null,
       item_id integer not null,
       primary key (user_id, item_id),
       foreign key (item_id) references items(id)
);
