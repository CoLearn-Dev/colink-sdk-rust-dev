#![allow(dead_code)]
use colink::{
    extensions::instant_server::{InstantRegistry, InstantServer},
    CoLink,
};

pub async fn set_up_test_env(
    num: usize,
) -> Result<
    (InstantRegistry, Vec<InstantServer>, Vec<CoLink>),
    Box<dyn std::error::Error + Send + Sync + 'static>,
> {
    let ir = InstantRegistry::new();
    let mut iss = vec![];
    let mut cls = vec![];
    for _ in 0..num {
        let is = InstantServer::new();
        let cl = is.get_colink().switch_to_generated_user().await?;
        iss.push(is);
        cls.push(cl);
    }
    Ok((ir, iss, cls))
}

pub async fn set_up_test_env_single_user() -> Result<
    (InstantRegistry, InstantServer, CoLink),
    Box<dyn std::error::Error + Send + Sync + 'static>,
> {
    let ir = InstantRegistry::new();
    let is = InstantServer::new();
    let cl = is.get_colink().switch_to_generated_user().await?;
    Ok((ir, is, cl))
}
