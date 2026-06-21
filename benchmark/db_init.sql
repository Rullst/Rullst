CREATE TABLE world (
    id integer NOT NULL,
    randomnumber integer NOT NULL DEFAULT 0
);

ALTER TABLE world ADD CONSTRAINT world_pkey PRIMARY KEY (id);

-- Insert exactly one record with ID 1 so our benchmarks don't fail fetching it
INSERT INTO world (id, randomnumber) VALUES (1, 42);
