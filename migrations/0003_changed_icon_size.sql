alter table items
drop column icon_url;

alter table items
add column icon_url varchar(256) not null;
