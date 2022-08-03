use structopt::StructOpt;
#[derive(Debug, StructOpt)]
#[structopt(name = "CoLink-SDK", about = "CoLink-SDK")]

pub struct CommandLineArgs {
    /// Address of CoLink server
    #[structopt(short, long)]
    pub addr: String,

    /// User JWT
    #[structopt(short, long)]
    pub jwt: String,

    /// Path to CA certificate.
    #[structopt(long)]
    pub ca: Option<String>,

    /// Path to client certificate.
    #[structopt(long)]
    pub cert: Option<String>,

    /// Path to private key.
    #[structopt(long)]
    pub key: Option<String>,
}
