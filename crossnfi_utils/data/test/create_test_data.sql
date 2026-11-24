
drop table if exists TEST.mplot;

CREATE TABLE test.mplot
       (cluster_name INTEGER NOT NULL,
        plot_name INTEGER NOT NULL,
        test_year INTEGER NOT NULL,
        marker_status INTEGER NOT null, 
        interval_name TEXT NOT NULL, 
        expected_implementation INTEGER NOT NULL);

-- relocation codes in the test example are 4 and 5

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

-- relocation_codes in the examples are 4 and 5

drop table if exists TEST.cluster cascade;

CREATE TABLE test.cluster 
       (id text primary key, 
        cluster_name bigint NOT NULL,
        test_year bigint NOT NULL,
        marker_status INTEGER NOT null, 
        lat  double precision not null,
        lon  double precision not null,
        expected_template  bigint not null, 
        expected_template_prev_cluster bigint);

insert into test.cluster values ('1', 12, 2012, 1, 0.0, 0.0, 12, null);
insert into test.cluster values ('2', 11, 2017, 4, 0.0, 0.0, 12, 12);
insert into test.cluster values ('3', 11, 2022, 2, 0.0, 0.0, 12, null);

insert into test.cluster values ('4', 21, 2012, 1, 1.01, 1.01, 21, null);
insert into test.cluster values ('5', 22, 2017, 4, 1.0, 1.0, 21, 21);
insert into test.cluster values ('6', 23, 2022, 5, 1.0, 1.0, 21, 22);

-- no relocation
insert into test.cluster values ('7', 31, 2012, 1, 2.0, 2.0, 31, null);
insert into test.cluster values ('8', 31, 2017, 2, 2.0, 2.0, 31, null);
insert into test.cluster values ('9', 31, 2022, 2, 2.0, 2.0, 31, null);

-- relcocation is in first year in the data"
insert into test.cluster values ('10', 41, 2012, 4, 3.0, 3.0, 41, null);
insert into test.cluster values ('11', 41, 2017, 2, 3.0, 3.0, 41, null);
insert into test.cluster values ('12', 41, 2022, 2, 3.0, 3.0, 41, null);


drop table if exists test.plot;

CREATE TABLE test.plot 
       (id text primary key, 
        cluster_id text NOT NULL references test.cluster,
        test_year bigint NOT NULL,
        expected_template  bigint not null, 
        expected_template_prev bigint);

insert into test.plot values ('1', '1', 2012, 12, null);
insert into test.plot values ('2', '2', 2012, 12, 12);
insert into test.plot values ('3', '5', 2017, 21, 21);
insert into test.plot values ('4', '6', 2017, 21, 22);
insert into test.plot values ('5', '9', 2022, 31, null);


drop table if exists test.plotsub;
create table test.plotsub 
       ( cluster_id1 bigint NOT NULL,
         cluster_id2 bigint NOT NUlL, 
         plot_id bigint NOT NULL, 
         test_year bigint NOT NULL, 
         status_cd bigint NOT NULL, 
         expected_nonsampled_status bigint);

-- nonsample and only forest
insert into test.plotsub values (1, 1, 1, 2012, 1, null);
insert into test.plotsub values (1, 1, 1, 2017, 5, 1);
insert into test.plotsub values (1, 1, 1, 2022, 1, null); 

-- nonsample and only nonforest 
insert into test.plotsub values (1, 1, 2, 2012, 2, null);
insert into test.plotsub values (1, 1, 2, 2017, 5, 2);
insert into test.plotsub values (1, 1, 2, 2022, 2, null); 

-- nonsample and both 
insert into test.plotsub values (1, 1, 3, 2012, 1, null);
insert into test.plotsub values (1, 1, 3, 2017, 5, 1);
insert into test.plotsub values (1, 1, 3, 2022, 2, null); 

-- only nonsample
insert into test.plotsub values (1, 1, 4, 2012, 5, 1);
insert into test.plotsub values (1, 1, 4, 2017, 5, 1);
insert into test.plotsub values (1, 1, 4, 2022, 5, 1); 

-- no nonsample forest
insert into test.plotsub values (1, 1, 5, 2012, 1, null);
insert into test.plotsub values (1, 1, 5, 2017, 1, null);
insert into test.plotsub values (1, 1, 5, 2022, 2, null); 


drop table if exists test.fiatree ;
create table test.fiatree 
       ( cluster_id1 bigint NOT NULL,
         cluster_id2 bigint NOT NUlL, 
         plot_id bigint NOT NULL,
         tree_id bigint NOT NULL, 
         test_year bigint NOT NULL, 
         status_cd bigint NOT NULL, 
         expected_id_use integer NOT NULL
       );

-- no dead tree
insert into test.fiatree values (1, 1, 1, 1, 2012, 1, 0);
insert into test.fiatree values (1, 1, 1, 1, 2017, 1, 0);
insert into test.fiatree values (1, 1, 1, 1, 2022, 1, 0);

-- only dead towards end
insert into test.fiatree values (1, 1, 1, 2, 2012, 1, 0);
insert into test.fiatree values (1, 1, 1, 2, 2017, 2, 0);
insert into test.fiatree values (1, 1, 1, 2, 2022, 2, 0);

-- dead in  between
insert into test.fiatree values (1, 1, 1, 3, 2012, 1, 0);
insert into test.fiatree values (1, 1, 1, 3, 2017, 2, 0);
insert into test.fiatree values (1, 1, 1, 3, 2022, 1, 1);

-- dead from the start 
insert into test.fiatree values (1, 1, 1, 4, 2012, 2, 0);
insert into test.fiatree values (1, 1, 1, 4, 2017, 2, 0);
insert into test.fiatree values (1, 1, 1, 4, 2022, 2, 0);


-- dead at start, then alive
insert into test.fiatree values (1, 1, 1, 5, 2012, 2, 0);
insert into test.fiatree values (1, 1, 1, 5, 2017, 1, 1);
insert into test.fiatree values (1, 1, 1, 5, 2022, 1, 1);


-- dead at start, alenating afterwards
insert into test.fiatree values (1, 1, 1, 6, 2012, 2, 0);
insert into test.fiatree values (1, 1, 1, 6, 2017, 1, 1);
insert into test.fiatree values (1, 1, 1, 6, 2022, 2, 1);
insert into test.fiatree values (1, 1, 1, 6, 2027, 1, 2);
insert into test.fiatree values (1, 1, 1, 6, 2032, 2, 2);
insert into test.fiatree values (1, 1, 1, 6, 2037, 1, 3);

-- alive at start, alenating afterwards
insert into test.fiatree values (1, 1, 1, 7, 2012, 1, 0);
insert into test.fiatree values (1, 1, 1, 7, 2017, 2, 0);
insert into test.fiatree values (1, 1, 1, 7, 2022, 1, 1);
insert into test.fiatree values (1, 1, 1, 7, 2027, 2, 1);
insert into test.fiatree values (1, 1, 1, 7, 2032, 1, 2);
insert into test.fiatree values (1, 1, 1, 7, 2037, 2, 2);


