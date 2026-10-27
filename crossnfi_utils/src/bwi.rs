use crate::read_connection_parameters;
use tokio_postgres::{ connect, NoTls, Client}; 
use super::TableNames;

pub(crate) async fn create_year_column(client: &Client, table_names: TableNames)
                            -> Result<u64, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let drop_query = format!(
        "alter table {schema}.{table} 
             drop column if exists {new_col};", 
        schema=table_names.schema_name, 
        table=table_names.table_name,
        new_col=table_names.year_column.as_ref().unwrap().as_str());
    client.execute(drop_query.as_str(), &[]).await?;
    
    let create_query = format!(
        "alter table {schema}.{table} 
             add column {new_col} integer default -1;", 
        schema=table_names.schema_name, 
        table=table_names.table_name,
        new_col=table_names.year_column.as_ref().unwrap().as_str());
    client.execute(create_query.as_str(), &[]).await?;
    

    let update_query = format!(
        "update {schema}.{table} t
         set {new_col}  =  cast(substring(t.{source_col} from \
                             '(?:bwi|ci)(.*)') as integer );", 
        schema=table_names.schema_name, 
        table=table_names.table_name,
        new_col=table_names.year_column.unwrap(), 
        source_col=table_names.source_column.unwrap());
    let response = client.execute(&update_query, &[]).await?;
    Ok(response)            
    }


