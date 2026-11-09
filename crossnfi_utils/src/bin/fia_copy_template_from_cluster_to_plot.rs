use crossnfi_utils::{TableNames, connect_crossnfi, template_cluster_to_plot};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send  + Sync + 'static>> {
    let (client, _conn_handle) =  connect_crossnfi().await?;
    
            let table_names = TableNames { 
            schema_name: "fia".to_string(), 
            table_name: "subplot".to_string(), 
                // foreign key to cluster
            cluster_columns: vec!["plt_cn".to_string()], 
            plot_column: None , 
            year_column: None,
                // cluster table, cluster table's primary, 
                // template source col, template_prev_cluster source col                
            source_columns: vec!["plot".to_string(), 
                                 "cn".to_string(), 
                                 "template".to_string(), 
                                 "template_prev_cluster".to_string()],
            };

    template_cluster_to_plot(&client, table_names).await?;    
    Ok(())

     
}
