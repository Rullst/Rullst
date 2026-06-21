CREATE TABLE IF NOT EXISTS world (
    id SERIAL PRIMARY KEY,
    text VARCHAR(255) NOT NULL
);

INSERT INTO world (id, text) VALUES (1, 'Hello World From DB');
