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
    user_id              SERIAL PRIMARY KEY,
    user_username        VARCHAR NOT NULL,
    user_name            VARCHAR NOT NULL,
    user_email           VARCHAR NOT NULL DEFAULT ' ',
    user_creation_date   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_last_login      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO users (user_username, user_name, user_email) VALUES
    ('marius', 'Màrius Montón', 'marius.monton@gmail.com');
;

CREATE TABLE status
(
    st_id            SERIAL PRIMARY KEY,
    st_title         VARCHAR NOT NULL DEFAULT ' ',
    st_description   VARCHAR NOT NULL DEFAULT ' ',
    st_short_name    VARCHAR NOT NULL DEFAULT ' '
);

INSERT INTO status (st_title, st_description, st_short_name) VALUES
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
    cat_id            SERIAL PRIMARY KEY,
    cat_title         VARCHAR NOT NULL DEFAULT ' ',
    cat_description   VARCHAR NOT NULL DEFAULT ' ',
    cat_tag           VARCHAR NOT NULL DEFAULT ' '
);

INSERT INTO categories (cat_title, cat_description, cat_tag) VALUES
    ('General', '', 'G'),
    ('System', '', 'SYS'),
    ('HW', '', 'HW'),
    ('SW', '', 'SW'),
    ('API', '', 'API');

CREATE TABLE verification
(
    verification_id             SERIAL PRIMARY KEY,
    verification_name           VARCHAR NOT NULL DEFAULT ' ',
    verification_description    VARCHAR NOT NULL DEFAULT ' '
);

INSERT INTO verification (verification_name, verification_description) VALUES
    ('Inspection', 'Nondestructive examination of a system'),
    ('Analisys', 'Verification of a product or system using models, calculations and testing equipment'),
    ('Demonstration', 'The manipulation of the product or system as it is intended to be used to verify that the results are as planned or expected.'),
    ('Test', 'Verification of a product or system using a controlled and predefined series of inputs, data, or stimuli ');

CREATE TABLE tests
(
    test_id             SERIAL PRIMARY KEY,
    test_name           VARCHAR NOT NULL DEFAULT ' ',
    test_reference      VARCHAR NOT NULL DEFAULT ' ',
    test_description    VARCHAR NOT NULL DEFAULT ' ',
    test_source         VARCHAR NOT NULL DEFAULT ' ',
    test_status         INTEGER NOT NULL DEFAULT 0,
    test_parent         INTEGER NOT NULL DEFAULT 0
);

