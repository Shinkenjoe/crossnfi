use crossnfi_utils::{TableNames, connect_crossnfi,implementation_identification};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send  + Sync + 'static>> {
    let (client, _conn_handle) =  connect_crossnfi().await?;
    let table_names = TableNames {
        schema_name : "fia".to_string(),
        table_name : "SUBPLOT".to_string(), 
        cluster_columns : vec!["STATECD".to_string(), 
                               "UNITCD".to_string(), 
                               "COUNTYCD".to_string(), 
                               "PLOT".to_string()],
        plot_column : Some("SUBP".to_string()),
        year_column : Some("INVYR".to_string()), 
        source_column : Some("POINT_NONSAMPLE_REASN_CD".to_string()),
    };
    implementation_identification(&client, table_names, vec![6, 7]).await?;
    Ok(())
}
