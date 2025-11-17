DROP TABLE requirements;
DROP TABLE categories;
DROP TABLE status;
DROP TABLE tests;
DROP TABLE matrix;
DROP TABLE users;
DROP TABLE verification;

CREATE TABLE requirements
(
    id              SERIAL PRIMARY KEY,
    title           VARCHAR NOT NULL DEFAULT ' ',
    description     VARCHAR NOT NULL DEFAULT ' ',
    verification_method_id    INTEGER NOT NULL DEFAULT 1,
    current_status_id  INTEGER NOT NULL DEFAULT 1,
    author_id          INTEGER NOT NULL DEFAULT 0,
    reviewer_id        INTEGER NOT NULL DEFAULT 0,
    req_link            VARCHAR NOT NULL DEFAULT ' ',
    reference_code       VARCHAR NOT NULL DEFAULT ' ',
    category_id        INTEGER NOT NULL DEFAULT 1,
    parent_id          INTEGER NOT NULL DEFAULT 0,
    creation_date   timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    update_date     timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deadline_date   timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO requirements (title, description, current_status_id, author_id, reference_code, category_id) VALUES
    ('Root management', '', 1, 1, '', 1);


CREATE TABLE users
(
    id              SERIAL PRIMARY KEY,
    username        VARCHAR NOT NULL,
    name            VARCHAR NOT NULL,
    email           VARCHAR NOT NULL DEFAULT ' ',
    creation_date   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_login      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO users (username, name, email) VALUES
    ('marius', 'Màrius Montón', 'marius.monton@gmail.com');
;

CREATE TABLE status
(
    id            SERIAL PRIMARY KEY,
    title         VARCHAR NOT NULL DEFAULT ' ',
    description   VARCHAR NOT NULL DEFAULT ' ',
    short_name    VARCHAR NOT NULL DEFAULT ' '
);

INSERT INTO status (title, description, short_name) VALUES
    ('Draft', 'The requirement is still being edited', 'Drf'),
    ('Proposal', 'The requirement is still to be approved', 'Pro'),
    ('Accepted', 'The requirement is accepted and must be processed', 'Acc'),
    ('Rejected', 'The requirement is not accepted', 'Rej'),
    ('Cancelled', 'The requirement is cancelled', 'Can'),
    ('Finished', 'The requirement is finished', 'Fsh'),
    ('Passed', 'The test has passed', 'Pass'),
    ('Failed', 'The test has failed', 'Fail');

CREATE TABLE categories
(
    id            SERIAL PRIMARY KEY,
    title         VARCHAR NOT NULL DEFAULT ' ',
    description   VARCHAR NOT NULL DEFAULT ' ',
    tag           VARCHAR NOT NULL DEFAULT ' '
);

INSERT INTO categories (title, description, tag) VALUES
    ('General', '', 'G'),
    ('System', '', 'SYS'),
    ('HW', '', 'HW'),
    ('SW', '', 'SW'),
    ('API', '', 'API');

CREATE TABLE verification
(
    id             SERIAL PRIMARY KEY,
    name           VARCHAR NOT NULL DEFAULT ' ',
    description    VARCHAR NOT NULL DEFAULT ' '
);

INSERT INTO verification (name, description) VALUES
    ('Inspection', 'Nondestructive examination of a system'),
    ('Analisys', 'Verification of a product or system using models, calculations and testing equipment'),
    ('Demonstration', 'The manipulation of the product or system as it is intended to be used to verify that the results are as planned or expected.'),
    ('Test', 'Verification of a product or system using a controlled and predefined series of inputs, data, or stimuli ');

CREATE TABLE tests
(
    id             SERIAL PRIMARY KEY,
    name           VARCHAR NOT NULL DEFAULT ' ',
    reference_code      VARCHAR NOT NULL DEFAULT ' ',
    description    VARCHAR NOT NULL DEFAULT ' ',
    source         VARCHAR NOT NULL DEFAULT ' ',
    status_id         INTEGER NOT NULL DEFAULT 0,
    parent_id         INTEGER NOT NULL DEFAULT 0
);

