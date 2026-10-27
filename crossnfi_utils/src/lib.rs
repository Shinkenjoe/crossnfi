use std::fs;
use std::io::Read;
use std::path::{PathBuf};
use tokio::task::JoinHandle;
use tokio_postgres::{connect, NoTls, Client};
//use tokio_postgres::row::Row;
// use std::any::Any;

mod bwi;


/// specifies the names of tables and columns to be modified by
/// the SQL modifiying functions

#[derive(Default, Debug)]
pub struct TableNames {    
    schema_name: String, 
    table_name: String, 
    cluster_column: Option<String>, 
    plot_column: Option<String>, 
    year_column: Option<String>, 
    source_column: Option<String>, 
}





/// identify plot or cluster installment - initialization phase


async fn installment_identification(
    client: &Client,
    table_names: TableNames,
    plot_movement_codes: Vec<i32>, )
    -> Result<u64, Box<dyn std::error::Error + Send + Sync + 'static>> {
    
    Ok(0)
}

struct ConnectionParameters {
    user : String, 
    password : String, 
    database : String,
}


/// connect to the crossnfi database
/// the connection details have to be specified in txt-files in the directory
/// "data/connection"
/// 
/// # Returns
/// The `tokio_postgres::Client` handle and the `JoinHandle` for the resource
/// managing task running in the background. 
async fn connect_crossnfi() -> Result<(Client, JoinHandle<()>),
                                       Box<dyn std::error::Error + 'static>> {
    let conn_param = read_connection_parameters();
    let conn_string = format!("user={} password={} dbname={} host=localhost", 
                              conn_param.user, conn_param.password, conn_param.database);
    let (client, connection) = connect(&conn_string, NoTls).await?;
    let conn_handle = tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("{e:?}");
        }
    });
    Ok((client, conn_handle))
}

/// create a ConnectionParameters struct by reading the text files in the
/// data directory
fn read_connection_parameters() -> ConnectionParameters {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut password_path = base.clone();
    password_path.push(PathBuf::from("data/connection/password.txt"));
    let mut password = String::new();
    fs::File::open(password_path)
        .expect("Password file not found as /data/connection/password.txt")
        .read_to_string(&mut password)
        .expect("Password file could not be read");
    let mut user_path = base.clone();
    user_path.push(PathBuf::from("data/connection/user.txt"));
    let mut user = String::new();
    fs::File::open(user_path)
        .expect("User file not found as /data/connection/user.txt")
        .read_to_string(&mut user)
        .expect("User file could not be read");
    let mut database_path = base.clone();
    database_path.push(PathBuf::from("data/connection/database.txt"));
    let mut database = String::new();
    fs::File::open(database_path)
        .expect("Database file not found as /data/connection/database.txt")
        .read_to_string(&mut database)
        .expect("Database file could not be read");
        ConnectionParameters {
            user : user.trim().to_string(),
            password : password.trim().to_string(),
            database : database.trim().to_string(),
        }
     
}




#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{rstest, fixture };
    use tokio::runtime::Runtime;
    
    #[fixture]
    #[once]
    fn conn_params() -> ConnectionParameters {
        read_connection_parameters()
    }
    

    async fn install_test_database_query(conn_params : &ConnectionParameters) {
        let (client, conn_handle) = connect_crossnfi().await.
            expect("Could not connect");
        let mut test_data_sql = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_data_sql.push(PathBuf::from("data/test/create_test_data.sql"));
        let mut test_dump = String::new();
        let bytes_read = fs::File::open(test_data_sql)
            .expect("Couldn't open test sql dump")
            .read_to_string(&mut test_dump)
            .expect("Couldn't read test sql dump");
        let statements = test_dump.split_inclusive(";")
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        for statement in statements {
            let rows_affected = client.execute(statement.as_str(), &[]).await
                .expect("Loading test dump did not work");
        }
       

        let result = client.query("SELECT * FROM test.mplot;", &[]).await.unwrap();


    }

    #[fixture]
    #[once]
    fn install_test_database(conn_params : &ConnectionParameters) -> i32 {
        let rt = Runtime::new().unwrap();
        rt.block_on(install_test_database_query(conn_params));
        3
    }

    #[fixture]
    async fn connect_testdb(install_test_database: &i32) -> Client {
        connect_crossnfi().await.
            expect("Could not connect to testdb").0
    }



    #[rstest]
    fn test_database(install_test_database : &i32){
        assert_eq!(*install_test_database, 3);
    }
    
    #[rstest]
    #[tokio::test]
    async fn test_add_bwi_year_column(connect_testdb : impl Future<Output = Client>) {
        let client = connect_testdb.await;
        let table_names = TableNames { 
            schema_name: "test".to_string(), 
            table_name: "mplot".to_string(), 
            cluster_column: None, 
            plot_column: None, 
            year_column: Some("inv_year".to_string()),
            source_column: Some("interval_name".to_string()),
        };
        bwi::create_year_column(&client, table_names).await
            .expect("create_year_column had an err");
        let rows = client.query(
            "select m.test_year, m.inv_year
             from test.mplot m;", &[])
        .await.expect("could not query test db after col creation");
        for row in rows {
            let orig : i32 = row.get(0);
            let deriv : i32 = row.get(1);
            assert_eq!(orig, deriv);
        }
    }
    
    #[rstest]
    #[tokio::test]
    async fn test_create_installments(connect_testdb : impl Future<Output = Client>) {
        let client = connect_testdb.await;
        let table_names = TableNames { 
            schema_name: "test".to_string(), 
            table_name: "mplot".to_string(), 
            cluster_column: Some("cluster_name".to_string()), 
            plot_column: Some("plot_name".to_string()), 
            year_column: Some("inv_year".to_string()),
            source_column: Some("interval_name".to_string()),
        };
        
        
        let rows = client.query(
            "select m.installement, m.expected_installment
             from test.mplot m;", &[])
        .await.expect("could not query test db after col creation");
        for row in rows {
            let orig : i32 = row.get(0);
            let deriv : i32 = row.get(1);
            assert_eq!(orig, orig);
        }

    }
}
