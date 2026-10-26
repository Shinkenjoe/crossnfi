-- psql -X --set ON_ERROR_STOP=on --single-transaction dbname < path_to_file

SET default_transaction_read_only = OFF;

CREATE SCHEMA extensions;


GRANT USAGE, CREATE 
    ON SCHEMA extensions 
    TO ddl_admin;

GRANT USAGE ON SCHEMA extensions TO  users;


ALTER DEFAULT PRIVILEGES IN SCHEMA extensions
  GRANT SELECT ON TABLES TO users;
ALTER DEFAULT PRIVILEGES IN SCHEMA extensions
  GRANT INSERT,UPDATE,DELETE ON TABLES TO devs;


ALTER DEFAULT PRIVILEGES IN SCHEMA extensions
  GRANT SELECT ON  SEQUENCES  TO users;
ALTER DEFAULT PRIVILEGES IN SCHEMA extensions
  GRANT UPDATE ON SEQUENCES TO devs;

ALTER DEFAULT PRIVILEGES IN SCHEMA extensions
  GRANT EXECUTE ON ROUTINES TO devs;

SET search_path TO extensions;



CREATE EXTENSION IF NOT EXISTS postgis WITH SCHEMA extensions;
-- could not install http extension on the host
-- CREATE extension IF NOT EXISTS http WITH SCHEMA extensions;
CREATE SCHEMA IF NOT EXISTS topology;
CREATE EXTENSION IF NOT EXISTS postgis_topology WITH SCHEMA topology;
-- add pg_jsonschema in schema extensions
-- could not install following extensions on the host
-- CREATE EXTENSION IF NOT EXISTS pg_jsonschema WITH SCHEMA extensions;
-- add moddatetime extension
-- create extension if not exists moddatetime schema extensions;


