use std::io::Write;

use camino::Utf8PathBuf as PathBuf;
use clap::Parser;
use color_eyre::eyre::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tera::Tera;
use yaml_front_matter::{Document, YamlFrontMatter};

static TEMPLATE: Lazy<Tera> = Lazy::new(|| {
    let mut tera = Tera::default();
    match tera.add_raw_templates(vec![("slides.html", include_str!("template.html"))]) {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };
    tera
});

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Presentation PDF (one slide per page)
    #[clap(parse(from_str), value_name = "FILE")]
    pdf: PathBuf,

    /// Slide annotations
    #[clap(parse(from_str), value_name = "FILE")]
    md: PathBuf,

    /// Output directory (basename)
    #[clap(short, long, parse(from_str), value_name = "DIR")]
    output: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
struct Metadata {
    title: String,
    date: String,
    location: String,
}

#[derive(Serialize)]
struct Context {
    metadata: Metadata,
    slides: Vec<String>,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    println!("Value for pdf: {}", cli.pdf);
    println!("Value for md: {}", cli.md);

    let output_path = if let Some(output_path) = cli.output.as_deref() {
        output_path
    } else {
        cli.pdf.file_stem().unwrap().into()
    };
    println!("Value for output: {}", output_path);
    std::fs::create_dir_all(output_path)?;

    let md = std::fs::read_to_string(cli.md)?;
    let document: Document<Metadata> = YamlFrontMatter::parse::<Metadata>(&md).unwrap();

    let product = Context {
        metadata: document.metadata,
        slides: vec!["1".into(), "2".into(), "3".into()],
    };
    let result = TEMPLATE.render("slides.html", &tera::Context::from_serialize(&product)?)?;

    let mut out_html = output_path.to_path_buf();
    out_html.push("slides.html");
    let mut fp = std::fs::File::create(out_html)?;
    fp.write_all(result.as_bytes())?;
    fp.flush()?;

    Ok(())
}
