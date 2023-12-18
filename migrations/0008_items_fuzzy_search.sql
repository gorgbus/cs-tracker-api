create table items (
    market_hash_name varchar(128) not null
);

create extension if not exists pg_trgm;

create index items_market_hash_name_fts_idx on items using gin(to_tsvector('english', market_hash_name));
