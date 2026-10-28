use std::fs;
use std::io::Read;
use std::path::{PathBuf};
use tokio::task::JoinHandle;
use tokio_postgres::{connect, NoTls, Client };
//use tokio_postgres::row::Row;
// use std::any::Any;

pub mod bwi;


/// specifies the names of tables and columns to be modified by
/// the SQL modifiying functions

#[derive(Default, Debug)]
pub struct TableNames {    
    pub schema_name: String, 
    pub table_name: String, 
    /// name of the column holding the cluster identifier
    pub cluster_columns: Vec<String>, 
    /// name of the column holding the plot identifier
    pub plot_column: Option<String>, 
    /// name of the column holding the year (either to be created or existant in the table) 
    pub year_column: Option<String>, 
    /// name for either
    /// - the column holding the codes for the case that the plot moved
    /// - the column from which the year was extracted
    pub source_column: Option<String>, 
}





/// identify plot or cluster implementation - initialization phase


pub async fn implementation_identification(
    client: &Client,
    table_names: TableNames,
    plot_movement_codes: Vec<i32>, )
    -> Result<u64, Box<dyn std::error::Error + Send + Sync + 'static>> {

    // explode table_names
    let TableNames { 
        schema_name : schema, 
        table_name  : table, 
        cluster_columns , 
        plot_column, 
        year_column, 
        source_column } = table_names;
    let year = year_column.expect("no year_column given");
    let plot_moved = source_column.expect("no column for plot moved codes given");
    let implementation = "implementation";

    let (cluster_plot, join_plot, luy_join_plot) = 
        create_cluster_plot_sql_fragments(cluster_columns,plot_column);

    // SQL list string to test for plot movement. Like "( 4 )" or "( 6, 7 )"
    if plot_movement_codes.is_empty() {
        return Err("plot movement codes not given".into()); 
    }
    let mut plot_move_codes = format!("( {}", plot_movement_codes[0]);
    for code in plot_movement_codes.iter().skip(1) {
        let code_string = format!(", {}", code);
        plot_move_codes.push_str(&code_string);
    }
    plot_move_codes.push_str(" )");
    
    // drop and create the column for holding the implementations
    let drop_query = format!(
        "alter table {schema}.{table} 
             drop column if exists {implementation};;");
    client.execute(&drop_query, &[]).await?;
    
    let create_query = format!(
        "alter table {schema}.{table} 
             add column {implementation} integer;");
    client.execute(&create_query, &[]).await?;

    
    // pre loop initialization
    // create a luy temporary table in which for each plot template the earliest year 
    // is noted, which has no implementation identification. 
    // in the first run befor the loop this includes the plots, 
    // where the plot has moved
    
    let luy_query = format!(
        "create temporary table least_unimplemented_year as
         (select {cluster_plot}, min({year}) luy 
          from {schema}.{table} m
          where m.{implementation} is null
          group by {cluster_plot});");
    client.execute(&luy_query, &[]).await?;

    // set the first implementation as the one in the years as defined 
    // in the luy temporary table
    
    let set_luy_query = format!(
        "update {schema}.{table} m
 	set {implementation} = 1
	from least_unimplemented_year o
	where {join_plot} and 
   	m.{year} = o.luy;"
    );
    client.execute(&set_luy_query, &[]).await?;
    
    // count how many unimplemented datasets are left
    let unimplemented_query = format!(
        "select count(*)
         from {schema}.{table} 
         where {implementation} is null;
         "
    );
    let mut unimplemented : i64  = client.query_one_scalar::<i64, String>(
        &unimplemented_query, 
        &[],
    ) .await?.into();

    // loop until there are no more plot dataset not aligned to a 
    // plot template implementation 

    while unimplemented > 0 {

        // drop luy table query, for recalculating luy
        let drop_luy_table_query = "drop table if exists least_unimplemented_year;";
        client.execute(drop_luy_table_query, &[]).await?;
        
        // least unimplemented year query can be reused from above
        client.execute(luy_query.as_str(), &[]).await?;

        // drop giy table query, for recalculating giy
        let drop_giy_table_query = "drop table if exists greatest_implemented_year;";
        client.execute(drop_giy_table_query, &[]).await?;

        // create a temporary table which holds for each plot template the latest year
        // for which there exists an implementation
        let giy_query = format!(
            "create temporary table greatest_implemented_year as
             (select {cluster_plot}, max({year}) giy, max({implementation}) imp
              from {schema}.{table} m
    	      where {implementation} is not null
              group by {cluster_plot});");    
        client.execute(&giy_query, &[]).await?;

        // drop assign_prev_implementation table query, for recalculating 
        let drop_assign_prev_inst_table_query = "
                          drop table if exists assign_prev_implementation;";
        client.execute(drop_assign_prev_inst_table_query, &[]).await?;

        // create a temporary table, where for each template plot's dataset 
        // with the least unimplemented year, you find the implementation of 
        // the dataset with the greatest implemented year 
        let assign_prev_implementation_query = format!(
            "create temporary table assign_prev_implementation as
	     (select {cluster_plot}, m.{year}, o.imp
	      from {schema}.{table} m 
	      join greatest_implemented_year o 
	        on {join_plot}
	      join least_unimplemented_year luy
                 on {luy_join_plot} and 
   	         m.{year} = luy.luy);");
        client.execute(&assign_prev_implementation_query, &[]).await?;

        // write the matchup found above into every dataset with a template plot's
        // least unimplemented year. 
        let set_prev_implementation_query = format!(
            "update {schema}.{table} m 
	     set {implementation} = o.imp
  	    from assign_prev_implementation o
 	    where {join_plot} and 
		  m.{year} = o.{year};
            ");
        client.execute(&set_prev_implementation_query, &[]).await?;
    
        // from the last edited datasets identify those 
        // which have a code for a plot move, 
        // mark the creation of a new plot main area  implementing this template plot 
        // by incrementing the implementation count. 
        let create_new_implementation_query = format!(
            "update {schema}.{table} m
	     set {implementation} = {implementation} + 1
             from assign_prev_implementation o
             where {join_plot} and 
      	       m.{year} = o.{year} and  
               m.{plot_moved} in  {plot_move_codes};");
        client.execute(&create_new_implementation_query, &[]).await?;
    
        // see how many unimplemented datasets are left
        unimplemented = client.query_one_scalar::<i64, String>(&unimplemented_query, &[])
            .await?.into();
    }    
    
    Ok(0)
}


/// for the cluster and plot combo we create a comma seperated string to inject 
/// into the select and group by clauses. 
/// we assume in the SQL queries the table is abbreviated with m. 
/// there is no comma at the end

/// for join conditions we need to use equality and we assume the other table
/// is abbreviated by 'o'
fn create_cluster_plot_sql_fragments(cluster_columns: Vec<String>, 
                                     plot_column : Option<String>, 
) -> (String, String, String) {
    
    let mut cluster_plot = String::new();
    let mut join_plot = String::new();
    let mut luy_join_plot = String::new();    
    for name in cluster_columns.iter() {
        let name_string = format!(" m.{} ,", name);
        cluster_plot.push_str(&name_string);
        let eq_string = format!("m.{0} = o.{0} and\n", name);
        join_plot.push_str(&eq_string);
        let luy_string = format!("m.{0} = luy.{0} and\n", name);
        luy_join_plot.push_str(&luy_string);

    }
    if let Some(plot_name) = plot_column {
        let plot_string = format!(" m.{} ", plot_name);
        cluster_plot.push_str(&plot_string);        
        let eq_string = format!(" m.{0} = o.{0} ", plot_name);
        join_plot.push_str(&eq_string);
        let luy_string = format!(" m.{0} = luy.{0} ", plot_name);
        luy_join_plot.push_str(&luy_string);

    // no plot, only cluster names, so delete the end comma 
    // and the end 'and' from the last cluster         
    } else { 
        cluster_plot.pop();
        join_plot.pop();
        join_plot.pop();
        join_plot.pop();
        join_plot.pop();
        join_plot.push('\n');
        luy_join_plot.pop();
        luy_join_plot.pop();
        luy_join_plot.pop();
        luy_join_plot.pop();
        luy_join_plot.push('\n');
    }
    (cluster_plot, join_plot, luy_join_plot)
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
pub async fn connect_crossnfi() -> Result<(Client, JoinHandle<()>),
                                       Box<dyn std::error::Error + Send + Sync + 'static>> {
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
    
    #[allow(unused_variables)]
    async fn install_test_database_query(conn_params : &ConnectionParameters) {
        let (client, _conn_handle) = connect_crossnfi().await.
            expect("Could not connect");
        let mut test_data_sql = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_data_sql.push(PathBuf::from("data/test/create_test_data.sql"));
        let mut test_dump = String::new();
        let _bytes_read = fs::File::open(test_data_sql)
            .expect("Couldn't open test sql dump")
            .read_to_string(&mut test_dump)
            .expect("Couldn't read test sql dump");
        let statements = test_dump.split_inclusive(";")
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        for statement in statements {
            let _rows_affected = client.execute(statement.as_str(), &[]).await
                .expect("Loading test dump did not work");
        }
       

        let _result = client.query("SELECT * FROM test.mplot;", &[]).await.unwrap();


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
        assert_eq!(*install_test_database, 3);
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
            cluster_columns: vec![], 
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
    async fn test_create_implementations(connect_testdb : impl Future<Output = Client>) {
        let client = connect_testdb.await;
        let table_names = TableNames { 
            schema_name: "test".to_string(), 
            table_name: "mplot".to_string(), 
            cluster_columns: vec!["cluster_name".to_string()], 
            plot_column: Some("plot_name".to_string()), 
            year_column: Some("test_year".to_string()),
            source_column: Some("marker_status".to_string()),
        };
        let plot_movement_codes = vec![4,5];
        implementation_identification(&client, table_names, plot_movement_codes).await
            .expect("something went wrong at identifying implementations");
        
        let rows = client.query(
            "select m.implementation, m.expected_implementation
             from test.mplot m;", &[])
        .await.expect("could not query test db after col creation");        
        for row in rows {
            let orig : i32 = row.get(0);
            let deriv : i32 = row.get(1);
            assert_eq!(orig, deriv);
        }
        
    }
    
    #[rstest]
    fn test_cluster_plot_sql_fragments() {
        let cluster_columns = vec!["STATECD".to_string(), 
                                   "UNITCD".to_string(), 
                                   "COUNTYCD".to_string(), 
                                   "PLOT".to_string()];
        let plot_column = Some("SUBP".to_string());
        let (cluster_plot, join_plot, luy_join_plot) = create_cluster_plot_sql_fragments(
            cluster_columns, plot_column);
        
        let exp_cluster_plot = " m.STATECD , m.UNITCD , m.COUNTYCD , m.PLOT , m.SUBP ";
        let exp_join_plot = "m.STATECD = o.STATECD and
m.UNITCD = o.UNITCD and
m.COUNTYCD = o.COUNTYCD and
m.PLOT = o.PLOT and
 m.SUBP = o.SUBP ";

        let exp_luy_join_plot = "m.STATECD = luy.STATECD and
m.UNITCD = luy.UNITCD and
m.COUNTYCD = luy.COUNTYCD and
m.PLOT = luy.PLOT and
 m.SUBP = luy.SUBP ";

        assert_eq!(cluster_plot, exp_cluster_plot);
        assert_eq!(join_plot, exp_join_plot);
        assert_eq!(luy_join_plot, exp_luy_join_plot);

    }
}
