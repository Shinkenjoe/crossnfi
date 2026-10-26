-- psql -X --set ON_ERROR_STOP=on  dbname < path_to_file
-- dbname should be the database of a superuser

DROP DATABASE IF EXISTS bwi;

CREATE DATABASE bwi; 

GRANT CONNECT ON DATABASE bwi TO users;
GRANT CREATE ON DATABASE bwi TO ddl_admin;
ALTER DATABASE bwi OWNER TO ddl_admin;

