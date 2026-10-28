
drop table if exists TEST.mplot;

CREATE TABLE test.mplot
       (cluster_name INTEGER NOT NULL,
        plot_name INTEGER NOT NULL,
        test_year INTEGER NOT NULL,
        marker_status INTEGER NOT null, 
        interval_name TEXT NOT NULL, 
        expected_implementation INTEGER NOT NULL);

-- plot moved codes in the test example are 4 and 5

-- some new implementations
INSERT INTO test.mplot values       (1, 1, 2012, 3, 'bwi2012'::text, 1);
INSERT INTO test.mplot values       (1, 1, 2017, 4, 'ci2017'::text,  2); 
INSERT INTO test.mplot values       (1, 1, 2022, 1, 'bwi2022'::text, 2);
INSERT INTO test.mplot values       (1, 1, 2027, 4, 'ci2027'::text,  3);   
INSERT INTO test.mplot values       (1, 1, 2032, 1, 'bwi2032'::text, 3);

-- start with a marker not found 
INSERT INTO test.mplot values       (1, 2, 2012, 4, 'bwi2012'::text, 1);
INSERT INTO test.mplot values       (1, 2, 2017, 5, 'ci2017'::text,  2); 
INSERT INTO test.mplot values       (1, 2, 2022, 1, 'bwi2022'::text, 2);
INSERT INTO test.mplot values       (1, 2, 2027, 4, 'ci2027'::text,  3); 
INSERT INTO test.mplot values       (1, 2, 2032, 1, 'bwi2032'::text, 3);

-- start later
INSERT INTO test.mplot values       (1, 3, 2017, 1, 'ci2017'::text,  1); 
INSERT INTO test.mplot values       (1, 3, 2022, 5, 'bwi2022'::text, 2);
INSERT INTO test.mplot values       (1, 3, 2027, 1, 'ci2027'::text,  2); 
INSERT INTO test.mplot values       (1, 3, 2032, 1, 'bwi2032'::text, 2);



