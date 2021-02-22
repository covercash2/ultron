-- This file should undo anything in `up.sql`
create temporary table inventory_backup (
       server_id text not null,
       user_id text not null,
       item_id integer not null,
       primary key (server_id, user_id, item_id),
       foreign key (item_id) references items(id)
);

insert into inventory_backup select server_id, user_id, item_id from inventory;

drop table inventory;

create table inventory (
       user_id text not null,
       item_id integer not null,
       primary key (user_id, item_id),
       foreign key (item_id) references items(id)
);

insert into inventory select user_id, item_id from inventory_backup;

drop table inventory_backup;
