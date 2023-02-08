mod common;
use colink::extensions::policy_module::{Action, Rule, TaskFilter};
use common::*;

#[tokio::test]
async fn test_policy_module() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let (_ir, _is, cl) = set_up_test_env_single_user().await?;
    //default policy
    let res = cl.policy_module_get_rules().await?;
    assert!(res.len() == 1);
    //test remove
    cl.policy_module_remove_rule(&res[0].rule_id).await?;
    let res = cl.policy_module_get_rules().await?;
    assert!(res.is_empty());
    //test add
    let task_filter = Some(TaskFilter {
        protocol_name: "greetings".to_string(),
        ..Default::default()
    });
    let action = Some(Action {
        r#type: "approve".to_string(),
        ..Default::default()
    });
    let priority = 1;
    let rule_id = cl
        .policy_module_add_rule(&Rule {
            task_filter: task_filter.clone(),
            action: action.clone(),
            priority: priority,
            ..Default::default()
        })
        .await?;
    let res = cl.policy_module_get_rules().await?;
    assert!(res.len() == 1);
    assert!(res[0].rule_id == rule_id);
    assert!(res[0].task_filter.eq(&task_filter));
    assert!(res[0].action.eq(&action));
    assert!(res[0].priority == priority);

    Ok(())
}
