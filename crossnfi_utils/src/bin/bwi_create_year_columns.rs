use crossnfi_utils::{TableNames, connect_crossnfi};
use crossnfi_utils::bwi::create_year_column;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send  + Sync + 'static>> {
    let (client, _conn_handle) =  connect_crossnfi().await?;
    let table_names = TableNames {
        schema_name : "inventory_archive".to_string(),
        table_name : "plot".to_string(), 
        cluster_columns : vec!["cluster_name".to_string()], 
        plot_column : Some("plot_name".to_string()),
        year_column : Some("inv_year".to_string()), 
        source_column : Some("interval_name".to_string()),
    };
    create_year_column(&client, table_names).await?;
    Ok(())
}
