use crossnfi_utils::{TableNames, connect_crossnfi, event_identification_by_code_edge};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send  + Sync + 'static>> {
    let (client, _) = connect_crossnfi().await?;
    let table_names = TableNames { 
        schema_name: "fia".to_string(), 
        table_name: "tree".to_string(), 
        cluster_columns: vec!["statecd".to_string(),
                              "unitcd".to_string(),
                              "countycd".to_string(),                             
                              "plot".to_string(), 
                              "subp".to_string()], 
        plot_column: Some("tree".to_string()), 
        year_column: Some("invyr".to_string()),
        source_columns: vec!["statuscd".to_string(), "id_use".to_string()],
    };
    let dead_codes = vec![2 as i64];
    let alive_codes = vec![1 as i64];
    event_identification_by_code_edge(&client, table_names,
                                      dead_codes, alive_codes).await.unwrap();
    Ok(())
}
