use crossnfi_utils::{connect_crossnfi};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send  + Sync + 'static>> {
    let (client, _) = connect_crossnfi().await?;
    let column_creation_query = "
    alter table inventory_archive.tree
      drop column if exists cluster_name, 
      drop column if exists plot_name, 
      add column cluster_name integer, 
      add column plot_name integer;
     ";
    client.execute(column_creation_query, &[]).await?;
    
    let update_query = "
        update inventory_archive.tree
            set cluster_name = p.cluster_name
        from inventory_archive.plot p
        where plot_id = p.id;";
    let update_query_2 = "
        update inventory_archive.tree
            set plot_name = p.plot_name
        from inventory_archive.plot p
        where plot_id = p.id;";
    
    

    client.execute(update_query, &[]).await?;
    client.execute(update_query_2, &[]).await?;

    Ok(())
}
