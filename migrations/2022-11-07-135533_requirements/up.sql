-- Your SQL goes here
CREATE TABLE requirements
(
    req_id              SERIAL PRIMARY KEY,
    req_title           VARCHAR NOT NULL DEFAULT ' ',
    req_description     VARCHAR NOT NULL DEFAULT ' ',
    req_current_status  INTEGER NOT NULL DEFAULT 1,
    req_author          VARCHAR NOT NULL DEFAULT ' ',
    req_author_email    VARCHAR NOT NULL DEFAULT ' ',
    req_link            VARCHAR NOT NULL DEFAULT ' ',
    req_reference       VARCHAR NOT NULL DEFAULT ' ',
    req_category        INTEGER NOT NULL DEFAULT 1,
    req_creation_date   timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    req_update_date     timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    req_deadline_date   timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO requirements (req_title, req_description, req_current_status, req_author, req_reference) VALUES
    ('The SW must manage requirements', 'Blablabla', 1, 'Màrius', 'REQ-SYS-010'),
    ('The SW must implement an API REST', 'API REST, blablabla', 1, 'Màrius', 'REQ-API-010' ),
    ('The SW must implement a Web app', 'Web app', 2, 'Màrius', 'REQ-SYS-020'),
    ('The Sw must export compliance matrix in excel format', 'Excel, blabla', 3, 'Victor', 'REQ-SYS-030');

CREATE TABLE status
(
    st_id     SERIAL PRIMARY KEY,
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
    ('Finished', 'The requirement is finished', 'Fsh');


CREATE TABLE categories 
(
    cat_id        SERIAL PRIMARY KEY,
    cat_title         VARCHAR NOT NULL DEFAULT ' ',
    cat_description   VARCHAR NOT NULL DEFAULT ' ',
    cat_tag           VARCHAR NOT NULL DEFAULT ' '
);

INSERT INTO categories (cat_title, cat_description, cat_tag)  VALUES 
    ('HW', '', 'HW'),
    ('SW', '', 'SW'),
    ('General', '', 'G'),
    ('System', '', 'SYS'); 

CREATE TABLE tests
(
    test_id SERIAL PRIMARY KEY,
    test_name VARCHAR NOT NULL DEFAULT ' ',
    test_description VARCHAR NOT NULL DEFAULT ' ',
    test_source INTEGER NOT NULL DEFAULT 0,
    test_status INTEGER NOT NULL DEFAULT 0
);

INSERT INTO tests (test_name, test_description, test_source, test_status) VALUES
    ('SW_Test_SPI', 'My nice test for SPI', 1, 3),
    ('SW_Test_I2C', 'My nice test for I2C', 1, 3),
    ('SW_Test_Comms', ' ', 2, 4),
    ('Unused Test', ' Nobody is expecting this test', 0, 3);

CREATE TABLE matrix
(
    matrix_req_id          INTEGER NOT NULL,
    matrix_test_id         INTEGER NOT NULL,
    matrix_creation_date   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (matrix_req_id, matrix_test_id)
);

INSERT INTO matrix (matrix_req_id, matrix_test_id) VALUES
    (1, 1), 
    (1, 2), 
    (2, 1), 
    (4, 1), 
    (4, 3);

