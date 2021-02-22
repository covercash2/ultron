-- This file should undo anything in `up.sql`
create temporary table items_backup(
       id integer not null primary key,
       name text not null,
       description text not null,
       emoji text not null,
       price integer not null,
       available integer
);

insert into items_backup select id, name, description, emoji, price, available from items;

drop table items;

create table items(
       id integer not null primary key,
       name text not null,
       description text not null,
       emoji text not null,
       price integer not null
);

insert into items select id, name, description, emoji, price from items_backup;

drop table items_backup;
