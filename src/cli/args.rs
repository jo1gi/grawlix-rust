use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Arguments {
    /// Path or link to comic book
    pub inputs: Vec<String>,
    /// Output template
    #[structopt(default_value = "{publisher}/{series}/{title}", short, long)]
    pub output_template: String,
}
