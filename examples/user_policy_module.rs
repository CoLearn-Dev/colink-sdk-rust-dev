use colink::extensions::policy_module::{Rule, TaskFilter};
use colink::CoLink;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let addr = &args[0];
    let jwt = &args[1];

    let cl = CoLink::new(addr, jwt);
    let res = cl.policy_module_get_rules().await?;
    println!("{:?}", res);

    let rule_id = cl
        .policy_module_add_rule(&Rule {
            task_filter: Some(TaskFilter {
                protocol_name: "greetings".to_string(),
                ..Default::default()
            }),
            action: "approve".to_string(),
            priority: 1,
            ..Default::default()
        })
        .await?;
    println!("rule_id: {}", rule_id);
    let res = cl.policy_module_get_rules().await?;
    println!("{:?}", res);

    cl.policy_module_remove_rule(&rule_id).await?;
    let res = cl.policy_module_get_rules().await?;
    println!("{:?}", res);

    Ok(())
}
