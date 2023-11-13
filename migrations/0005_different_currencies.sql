create type currencies as enum ('USD', 'EUR', 'CNY');

alter table investments
add column currency currencies default 'USD';
