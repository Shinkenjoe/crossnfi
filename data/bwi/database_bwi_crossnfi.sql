-- psql -X --set ON_ERROR_STOP=on  dbname < path_to_file
-- dbname should be the database of a superuser

DROP DATABASE IF EXISTS crossnfi;

CREATE DATABASE crossnfi; 

GRANT CONNECT ON DATABASE crossnfi TO users;
GRANT CREATE ON DATABASE crossnfi TO ddl_admin;
ALTER DATABASE crossnfi OWNER TO ddl_admin;

