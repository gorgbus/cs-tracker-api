drop table investments;
drop table collections;

create table collections (
    col_id int generated always as identity primary key,
    steam_id varchar(18) not null,
    name varchar(256) not null,
    constraint fk_owner_col
        foreign key (steam_id)
        references users (steam_id)
);

create table investments (
    inv_id serial primary key,
    steam_id varchar(18) not null,
    item varchar(128) not null,
    collection int not null,
    cost numeric(10, 2) not null,
    amount int not null,
    currency currencies default 'USD',
    constraint fk_owner_inv
    foreign key (steam_id)
    references users (steam_id),
    constraint fk_item_inv
    foreign key (item)
    references items (market_hash_name),
    constraint fk_collection_inv
    foreign key (collection)
    references collections (col_id)
);
