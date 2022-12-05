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
    req_parent          INTEGER NOT NULL DEFAULT 0,
    req_creation_date   timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    req_update_date     timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    req_deadline_date   timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO requirements (req_title, req_description, req_current_status, req_author, req_reference, req_category) VALUES
    ('The SW must manage requirements', 'Blablabla', 3, 'Màrius', 'REQ-SYS-010', 1),
    ('The SW must implement an API REST', 'API REST, blablabla', 3, 'Màrius', 'REQ-API-010' , 5),
    ('The SW must implement a Web app', 'Web app', 3, 'Màrius', 'REQ-SYS-020', 2),
    ('The SW must export compliance matrix in excel format', 'Excel, blabla', 3, 'Victor', 'REQ-SYS-030', 1);

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

CREATE TABLE tests
(
    test_id             SERIAL PRIMARY KEY,
    test_name           VARCHAR NOT NULL DEFAULT ' ',
    test_description    VARCHAR NOT NULL DEFAULT ' ',
    test_source         VARCHAR NOT NULL DEFAULT ' ',
    test_status         INTEGER NOT NULL DEFAULT 0
);

INSERT INTO tests (test_name, test_description, test_source, test_status) VALUES
    ('SW_Test_SPI', 'My nice test for SPI', 'my_test_spi.c:32', 7),
    ('SW_Test_I2C', 'My nice test for I2C', 'my_test_i2c.c:32', 7),
    ('SW_Test_Comms', ' ', 'Test Report.PDF', 8),
    ('Unused Test', ' Nobody is expecting this test', 'Some email', 7);

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

