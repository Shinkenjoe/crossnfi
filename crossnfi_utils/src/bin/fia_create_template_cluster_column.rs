use crossnfi_utils::{TableNames, connect_crossnfi, template_identification};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send  + Sync + 'static>> {
    let (client, _conn_handle) =  connect_crossnfi().await?;
    let table_names = TableNames {
        schema_name : "fia".to_string(),
        table_name : "PLOT".to_string(), 
        cluster_columns : vec!["STATECD".to_string(), 
                               "UNITCD".to_string(), 
                               "COUNTYCD".to_string(), 
                               "PLOT".to_string()],
        plot_column : None,
        year_column : Some("INVYR".to_string()), 
        source_columns : vec!["cn".to_string(), 
                              "kindcd".to_string(), 
                              "lat".to_string(), 
                              "lon".to_string(), 
                              "plot".to_string()],
    };
    template_identification(&client, table_names, 0.015, vec![3]).await?;


    
    // data cleaning, where the algorithm failed, because the tolerance of 0.015 was not 
    // appropriate (either too small or too big)
    let plot_68_query = "update fia.plot 
                   set template = plot, template_prev_cluster = null
                   where 
                      (statecd = 22 
                       and unitcd = 5 
                       and countycd = 49
                       and plot = 68);";
    client.execute(plot_68_query, &[]).await
        .expect("data cleaning plot 22_5_49_68");

    let plot_72_prev_query = "update fia.plot 
                   set template_prev_cluster = 24
                   where 
                      (statecd = 22 
                       and unitcd = 5 
                       and countycd = 49
                       and plot = 72
                       and kindcd = 3);";
    client.execute(plot_72_prev_query, &[]).await
        .expect("data cleaning plot 22_5_49_72");


    // template_prev_plt -22_5_49_68 2005 = null
    // template_prev_plt 22_5_49_72_2013 = 24

    let template_90056_query = "
                   update fia.plot 
                   set template = 90056
                   where statecd = 22 
                       and unitcd = 5 
                       and countycd = 15
                       and plot = 99;";
    client.execute(template_90056_query, &[]).await
        .expect("data cleaning plot 22_5_15_99 failed");

    let template_90056_prev_query = "
                   update fia.plot 
                   set template_prev_cluster = 90056
                   where statecd = 22 
                       and unitcd = 5 
                       and countycd = 15
                       and plot = 99
                       and kindcd = 3;";
    client.execute(template_90056_prev_query, &[]).await
        .expect("data cleaning plot 22_5_15_99 failed");



    let template_90036_query = "
                   update fia.plot 
                   set template = 90036
                   where statecd = 22 
                       and unitcd = 2 
                       and countycd = 9
                       and plot = 97;";
    client.execute(template_90036_query, &[]).await
        .expect("data cleaning plot 22_5_15_97 failed");

        let template_90036_prev_query = "
                   update fia.plot 
                   set template_prev_cluster = 90036
                   where statecd = 22 
                       and unitcd = 2 
                       and countycd = 9
                       and plot = 97
                       and kindcd = 3;";
    client.execute(template_90036_prev_query, &[]).await
        .expect("data cleaning plot 22_5_15_97 failed");



    Ok(())
}
