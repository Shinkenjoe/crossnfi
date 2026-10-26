-- create a schema in potsgres to load a fia state database into


DROP SCHEMA IF EXISTS fia_ca CASCADE;
CREATE SCHEMA fia_ca; 
GRANT USAGE, CREATE 
    ON SCHEMA fia_ca 
    TO ddl_admin;

GRANT USAGE ON SCHEMA fia_ca TO  users;
ALTER SCHEMA fia_ca OWNER TO ddl_admin;

ALTER DEFAULT PRIVILEGES IN SCHEMA fia_ca
  GRANT SELECT ON TABLES TO users;
ALTER DEFAULT PRIVILEGES IN SCHEMA fia_ca
  GRANT INSERT,UPDATE,DELETE ON TABLES TO devs;

ALTER DATABASE fia SET search_path to fia_ca, public;
