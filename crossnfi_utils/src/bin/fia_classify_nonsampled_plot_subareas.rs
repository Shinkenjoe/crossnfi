use crossnfi_utils::{TableNames, connect_crossnfi, 
                     classify_nonsampled_plot_subareas,
                     fia_create_plot_subarea_view};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send  + Sync + 'static>> {
    let (client, _conn_handle) =  connect_crossnfi().await?;
    fia_create_plot_subarea_view(&client).await?;
    let table_names = TableNames {
        schema_name : "fia".to_string(),
        table_name : "plot_subarea".to_string(), 
        cluster_columns : vec!["STATECD".to_string(), 
                               "UNITCD".to_string(), 
                               "COUNTYCD".to_string(), 
                               "PLOT".to_string()],
        plot_column : Some("subp".to_string()),
        year_column : Some("INVYR".to_string()), 
        source_columns : vec!["subp_cond".to_string(), "cond_status_cd".to_string()],
    };
    classify_nonsampled_plot_subareas(&client, 
                                      table_names,
                                      vec![1], 
                                      vec![2, 3, 4],
                                      vec![5]).await?;
    Ok(())
}
