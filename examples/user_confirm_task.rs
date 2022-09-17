use colink::CoLink;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let addr = &args[0];
    let jwt = &args[1];
    let task_id = &args[2];
    let action = if args.len() > 3 { &args[3] } else { "approve" };
    let cl = CoLink::new(addr, jwt);
    if action == "approve" {
        cl.confirm_task(task_id, true, false, "").await?;
    } else if action == "reject" {
        cl.confirm_task(task_id, false, true, "").await?;
    } else if action == "ignore" {
        cl.confirm_task(task_id, false, false, "").await?;
    } else {
        Err("Action not supported.")?;
    }

    Ok(())
}
