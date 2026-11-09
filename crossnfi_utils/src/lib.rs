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
    /// - the column holding the codes for the relocation case
    /// - the column from which the year was extracted (create_year_column)
    /// - the pair of columns holding the location of a template cluster (lon and lat)
    /// - the column from which to take the template cluster identification code
    pub source_columns: Vec<String>, 
}


/// identify a template cluster given a table of cluster datasets holding id references 
/// of cluster area sets 
///
/// # Scenario
/// a table holds records of cluster (or plot) datasets and id refererces to cluster area 
/// sets ( or plot main areas). The cluster template (or plot template) are not identified
/// only cluster area sets. When a field crew cannot identfy the reference position of 
/// a cluster area set, it notes down a  code (`relocation_codes`) in a special column 
/// , and create a new cluster area set implementing that template cluster.
/// Cluster area sets implement the same template cluster if their locations are sufficiently
/// close ot each other. This method treats cluster area sets which are within a distance of 
/// degrees (lon, lat) given by the argument `degree_tolerance` as implementing the same 
/// template cluster. It creates an identifying code in a column named "template" from the
/// cluster area set, which implement a template cluster earliest (by year). 
/// ## table_names.source_columns
/// holds 4 column names : 
/// 1. column holding the relocation code
/// 2. location latidude / y - coordinate
/// 3. location longitude / x - coordinate 
/// 4. column name which to use for creating the template cluster code

#[allow(unused_variables)]
pub async fn template_identification(
    client: &Client, 
    table_names: TableNames, 
    degree_tolerance : f64, 
    relocation_codes: Vec<i32>,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync + 'static>> {

    let TableNames { 
        schema_name : schema, 
        table_name  : table, 
        cluster_columns , 
        plot_column, 
        year_column, 
        source_columns 
    } = table_names;
    let year = year_column.expect("no year_column given");
    let [id, relocation_column, lat, lon, template_source] = source_columns.try_into()
        .expect("table_names.source_columns: id, relocation_column, lat, lon, template_src");
    let template = "template";  
    let template_prev_cluster = "template_prev_cluster";

    // in the data all cluster datasets of same plot code have the same location. 
    // we assume that the relocation code (kindcd = 3) is given in all cases of 
    // a relocation, that is  all cluster area sets for which the code is not apperating 
    // are the sole implementors of their template cluster. 
    // 1. identify the datasets which have relocations,
    // 3. for those datasets find datasets implementing the same template cluster 
    //    that is which have a location at a distance below `tolerance` 
    // 3. for all cluster datasets implementing one plot template set the column "template"
    //    to the cluster area set code of the earliest (by year) code         


        // SQL list string to test for plot movement. Like "( 4 )" or "( 6, 7 )"
    if relocation_codes.is_empty() {
        return Err("relocation codes are not given".into()); 
    }
    let mut relocation = format!("( {}", relocation_codes[0]);
    for code in relocation_codes.iter().skip(1) {
        let code_string = format!(", {}", code);
        relocation.push_str(&code_string);
    }
    relocation.push_str(" )");
    
    // drop and create the column for holding the templates
    let drop_query = format!(
        "alter table {schema}.{table} 
             drop column if exists {template};");
    client.execute(&drop_query, &[]).await?;
    
    let create_query = format!(
        "alter table {schema}.{table} 
             add column {template} bigint;");
    client.execute(&create_query, &[]).await?;

    // drop and create the column holding the previous plot main area 
    // implementing a template in case of a relocation
    let drop_query = format!(
        "alter table {schema}.{table} 
             drop column if exists {template_prev_cluster};");
    client.execute(&drop_query, &[]).await?;
    
    let create_query = format!(
        "alter table {schema}.{table} 
             add column {template_prev_cluster} bigint;");
    client.execute(&create_query, &[]).await?;


    
    // create sql fragments for select clauses and joining conditions
    let (cluster_plot, join_plot, _) = 
        create_cluster_plot_sql_fragments(cluster_columns,plot_column);

    
    // select the locations of cluster datasets holding the relocation code
    let reloc_locations_query = format!(
        "select {lat}, {lon} 
         from {schema}.{table}
         where {relocation_column} in {relocation};");
    let relocation_locations = client.query(&reloc_locations_query, &[])
        .await.expect("error at retrieving locations of relocations");
    for row in relocation_locations {
        
        let rel_lat : f64 = row.try_get(0).expect("could not retrieve relocation latitude");
        let rel_lon : f64 = row.try_get(1).expect("could not retrieve relocation longitude");
        let lower_lat = rel_lat - degree_tolerance;
        let upper_lat = rel_lat + degree_tolerance;
        let lower_lon = rel_lon - degree_tolerance;
        let upper_lon = rel_lon + degree_tolerance;
        
        let drop_reloc_table_query = format!(
            "drop table if exists relocs");

        let reloc_table_query = format!(
            " create temporary table relocs as
              (select m.{id}, {cluster_plot}, m.{year}, m.{relocation_column}
               from {schema}.{table} m
               where 
                   (m.{lat} between $1 and $2) and 
                   (m.{lon} between $3 and $4)
               order by {year});"
            );
        client.execute(&drop_reloc_table_query, &[]).await.
            expect("could not drop temporary reloc table");
        client.execute(&reloc_table_query, 
                       &[&lower_lat, &upper_lat, &lower_lon, &upper_lon]).await
        .expect("could not create temporary reloc table");

        let min_year_query = format!("select min({year}) from relocs;");
        let min_year : i64 = match client.query_one_scalar::<i64, _>(&min_year_query, &[])
            .await {
                Err(e) => { 
                    eprintln!{"{rel_lat}, {rel_lon}, {e}"};
                    panic!{"could not retrieve reloc min year"};
                }
                Ok(yr) => yr
            };

        let cluster_id_query = format!(
            "select min({template_source})
             from relocs
             where {year} = {min_year};");
        let template_id: i64  = client.query_one_scalar(&cluster_id_query, &[]).await
            .expect("could not retrieve template id");

        let update_relocated_query = format!(
            "update {schema}.{table} m
             set {template} = {template_id} 
             from relocs o
             where m.{id} = o.{id};");
        client.execute(&update_relocated_query, &[]).await
            .expect("could not set plot template id for relocation cases");                

        // set the column holding the previous plot main area 
        // implementing a template in case of a relocation

        let reloc_years_query = format!("
            select {year} 
            from relocs 
            where {relocation_column} in {relocation};");
        let reloc_years = client.query(&reloc_years_query, &[])
            .await.expect("could not collect years of relocations");
        for row in reloc_years {
            let reloc_year  = if let Some(rel_year) = row
                .try_get::<usize, Option<i64>>(0) 
                .expect("there was not a Some to unwrap for a reloc_year"){
                    rel_year
                } else {
                    panic!("there was not a Some to unwrap for a reloc_year");
                };
            
            let reloc_id_query = format!("                
                select {id} 
                from relocs 
                where {year} = {reloc_year};");
            let reloc_id : String = client.query_one_scalar(&reloc_id_query, &[])
                .await.expect("could not retrieve id of relocation cluster dataset");

            let prev_cluster_query = format!("
                select {template_source} 
                from relocs 
                where {year} = (select max({year})
                                from relocs 
                                 where {year} < {reloc_year});");
            let prev_cluster_option : Option<Option<i64>>  = client
                .query_opt_scalar(&prev_cluster_query, &[]).await
                .expect("could not query for plot previous of relocation");
            match prev_cluster_option {
                Some(Some(prev_cluster)) => {
                   let update_template_prev_cluster_query = format!(
                       "update {schema}.{table} 
                        set {template_prev_cluster} = {prev_cluster} 
                        where {id} = '{reloc_id}';"); 
                    client.execute(&update_template_prev_cluster_query, &[])
                        .await.expect("could not update template prev cluster");
                }
                Some(None) | None => {}
                
            };
        }
    }

    let update_nonrelocated_query = format!(
        " update {schema}.{table} 
          set {template} = {template_source}
          where {template} is null;");
    client.execute(&update_nonrelocated_query, &[]).await
        .expect("could not set plot template id for non-relocation cases");   
    Ok(0)
}    

/// copy over template id from cluster area set table into plot main area table 

#[allow(unused_variables)]
pub async fn template_cluster_to_plot(
    client: &Client, 
    table_names: TableNames, 
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

    let TableNames { 
        schema_name : schema, 
        table_name  : table, 
        cluster_columns , 
        plot_column, 
        year_column, 
        source_columns } = table_names;
    let cluster_column = cluster_columns.get(0)
        .expect("could not access cluster foreign key");
    let [cluster_table, 
         cluster_primary, 
         template_source, 
         template_prev_cluster_source] = source_columns
        .try_into()
        .expect("table_names.source_columns: cluster_table, cluster_primary, template_src");
    let template = "template";  
    let template_prev = "template_prev";

    let drop_query = format!(
        "alter table {schema}.{table} 
             drop column if exists {template};");
    client.execute(&drop_query, &[]).await?;
    
    let create_query = format!(
        "alter table {schema}.{table} 
             add column {template} bigint;");
    client.execute(&create_query, &[]).await?;

    let drop_query = format!(
        "alter table {schema}.{table} 
             drop column if exists {template_prev};");
    client.execute(&drop_query, &[]).await?;
    
    let create_query = format!(
        "alter table {schema}.{table} 
             add column {template_prev} bigint;");
    client.execute(&create_query, &[]).await?;

        
    let copy_query = format!(
        "update {schema}.{table} p
         set {template} = c.{template_source}, 
             {template_prev} = c.{template_prev_cluster_source}
         from {schema}.{cluster_table} c
         where c.{cluster_primary} = p.{cluster_column};
        ");
    client.execute(&copy_query, &[]).await
        .expect("copying template from cluster to plot failed");
    Ok(())
}






/// identify plot (or cluster) implementataion 
/// scenario you have a set of plot datasets referring to plot templates in a 
/// identifying key. The relocation of a plot main area, thus the creation of a 
/// new plot main area is marked by values (`relocation_codes`) a special column
/// (`table_names.source_column`) takes, further on called relocation code. 
/// the function  installs a column named `implementataion` in the table which is
/// used as identifier for different plot main area implementing a plot template. 
/// the plot dataset belonging to a plot template with the earliest inventory year
/// gets dubbed as implementation '1'. When a relocation code appears,
/// the implementation number gest incremented by 1 (a new plot main area gets created). 
/// # Returns
/// Result, holding the number of rows affected in the `Ok` variant or an error in `Err` 
pub async fn implementation_identification(
    client: &Client,
    table_names: TableNames,
    relocation_codes: Vec<i32>, )
    -> Result<u64, Box<dyn std::error::Error + Send + Sync + 'static>> {

    // explode table_names
    let TableNames { 
        schema_name : schema, 
        table_name  : table, 
        cluster_columns , 
        plot_column, 
        year_column, 
        source_columns } = table_names;
    let year = year_column.expect("no year_column given");
    let plot_moved = source_columns.get(0)
        .expect("no column for plot relocation codes given");
    let implementation = "implementation";

    let (cluster_plot, join_plot, luy_join_plot) = 
        create_cluster_plot_sql_fragments(cluster_columns,plot_column);

    // SQL list string to test for plot movement. Like "( 4 )" or "( 6, 7 )"
    if relocation_codes.is_empty() {
        return Err("plot movement codes not given".into()); 
    }
    let mut relocation = format!("( {}", relocation_codes[0]);
    for code in relocation_codes.iter().skip(1) {
        let code_string = format!(", {}", code);
        relocation.push_str(&code_string);
    }
    relocation.push_str(" )");
    
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
               m.{plot_moved} in  {relocation};");
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
    use tokio::{task};
    
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
        task::block_in_place(||{
            let handle = tokio::runtime::Handle::current();
            handle.block_on(install_test_database_query(conn_params));

        });
        3
    }

    #[fixture]
    async fn connect_testdb(install_test_database: &i32) -> Client {
        assert_eq!(*install_test_database, 3);
        connect_crossnfi().await.
            expect("Could not connect to testdb").0
    }



    #[rstest]
    #[tokio::test(flavor = "multi_thread", worker_threads=2)]
    async fn test_database(install_test_database : &i32){
        assert_eq!(*install_test_database, 3);
    }
    
    #[rstest]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_add_bwi_year_column(connect_testdb : impl Future<Output = Client>) {
        let client = connect_testdb.await;
        let table_names = TableNames { 
            schema_name: "test".to_string(), 
            table_name: "mplot".to_string(), 
            cluster_columns: vec![], 
            plot_column: None, 
            year_column: Some("inv_year".to_string()),
            source_columns: vec!["interval_name".to_string()],
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
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_identify_implementations(connect_testdb : impl Future<Output = Client>) {
        let client = connect_testdb.await;
        let table_names = TableNames { 
            schema_name: "test".to_string(), 
            table_name: "mplot".to_string(), 
            cluster_columns: vec!["cluster_name".to_string()], 
            plot_column: Some("plot_name".to_string()), 
            year_column: Some("test_year".to_string()),
            source_columns: vec!["marker_status".to_string()],
        };
        let relocation_codes = vec![4,5];
        implementation_identification(&client, table_names, relocation_codes).await
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

    #[rstest]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[allow(unused_variables)]
    async fn test_identify_templates(connect_testdb : impl Future<Output = Client>) {
        let client = connect_testdb.await;
        let table_names = TableNames { 
            schema_name: "test".to_string(), 
            table_name: "cluster".to_string(), 
            cluster_columns: vec!["cluster_name".to_string()], 
            plot_column: None , 
            year_column: Some("test_year".to_string()),
            source_columns: vec!["id".to_string(),
                                 "marker_status".to_string(), 
                                 "lat".to_string(),
                                 "lon".to_string(), 
                                 "cluster_name".to_string()],
        };
        let relocation_codes = vec![4,5];
        template_identification(&client, table_names, 0.02f64, relocation_codes).await
            .unwrap();
        
        let rows = client.query(
            "select c.template, c.expected_template, 
                    c.template_prev_cluster, c.expected_template_prev_cluster
             from test.cluster c;", &[])
        .await.expect("could not query test db after col creation");        
        for row in rows {
            let orig : i64 = row.try_get(0).expect("orig in test could not be found");
            let deriv : i64 = row.try_get(1).expect("deriv in test could not be found");
            let orig_prev : Option<i64> = row.try_get(2)
                .expect("orig_prev in test could not be found");
            let deriv_prev : Option<i64> = row.try_get(3)
                .expect("deriv_prev in test could not be found");
            assert_eq!(orig, deriv);
            
            assert_eq!(orig_prev, deriv_prev);
        }        
    }
    
    #[rstest]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[allow(unused_variables)]
    async fn test_template_cluster_to_plot(connect_testdb : impl Future<Output=Client>){
        let client = connect_testdb.await;
        let table_names = TableNames { 
            schema_name: "test".to_string(), 
            table_name: "plot".to_string(), 
            cluster_columns: vec!["cluster_id".to_string()], 
            plot_column: None , 
            year_column: None ,
            source_columns: vec!["cluster".to_string(), 
                                 "id".to_string(), 
                                 "expected_template".to_string(),
                                 "expected_template_prev_cluster".to_string()],
        };
        
        template_cluster_to_plot(&client, table_names)
            .await.expect("test copy template from cluster to plot failed");
        
        let rows = client.query(
            "select p.template, p.expected_template, 
                    p.template_prev, p.expected_template_prev
             from test.plot p;", &[])
        .await.expect("could not query test db after col creation");        
        for row in rows {
            let orig : i64 = row.try_get(0).expect("orig in test could not be found");
            let deriv : i64 = row.try_get(1).expect("deriv in test could not be found");
            let orig_prev : Option<i64> = row.try_get(2)
                .expect("orig_prev in test could not be found");
            let deriv_prev : Option<i64> = row.try_get(3)
                .expect("deriv_prev in test could not be found");
            assert_eq!(orig, deriv);            
            assert_eq!(orig_prev, deriv_prev);
        }
    } 
}
