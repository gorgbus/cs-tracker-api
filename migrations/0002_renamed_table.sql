drop table investements;

create table investments (
    inv_id serial primary key,
    steam_id varchar(18) not null,
    item varchar(128) not null,
    collection int not null,
    cost float not null,
    amount int not null,
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
