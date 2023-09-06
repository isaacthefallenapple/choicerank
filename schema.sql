CREATE TABLE ballot (
    id serial primary key,
    title varchar(63) not null,
    choices text not null,
    max_choices integer not null
);

CREATE TABLE ranking (
    id serial references ballot(id),
    name varchar(63),
    ranking text not null
);

-- CREATE TABLE IF NOT EXISTS result (
-- );
