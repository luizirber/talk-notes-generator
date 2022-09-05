use std::io::Write;

use camino::Utf8PathBuf as PathBuf;
use clap::Parser;
use color_eyre::eyre::Result;
use graphicsmagick::wand::MagickWand;
use once_cell::sync::Lazy;
use pulldown_cmark::{html, Options, Parser as MdParser};
use serde::{Deserialize, Serialize};
use tera::Tera;
use tracing::{info, instrument};
use yaml_front_matter::{Document, YamlFrontMatter};

static TEMPLATE: Lazy<Tera> = Lazy::new(|| {
    let mut tera = Tera::default();
    match tera.add_raw_templates(vec![("slides.html", RAW_HTML)]) {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };
    tera.autoescape_on(vec![]);
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
    link: String,
    event_name: String,
}

#[derive(Serialize)]
struct Context {
    metadata: Metadata,
    slides: Vec<String>,
}

#[instrument]
fn main() -> Result<()> {
    install_tracing();
    color_eyre::install()?;
    graphicsmagick::initialize();

    let cli = Cli::parse();

    let output_path = if let Some(output_path) = cli.output.as_deref() {
        output_path
    } else {
        cli.pdf.file_stem().unwrap().into()
    };
    info!("Saving output to {}/", output_path);
    std::fs::create_dir_all(output_path)?;

    let mut thumb_path = output_path.to_path_buf();
    thumb_path.push("thumbs");

    let mut thumb = thumb_path.clone();
    thumb.push("%0d.jpg");

    info!(
        "Processing PDF to generate thumbnails at {}/thumbs/",
        output_path
    );
    let mut mw = MagickWand::new();
    mw.read_image(cli.pdf.as_str())?
        .write_images(thumb.as_str(), 0)?;

    info!("Reading Markdown notes for each slide");
    let md = std::fs::read_to_string(cli.md)?;
    let document: Document<Metadata> = YamlFrontMatter::parse::<Metadata>(&md).unwrap();

    let options = Options::empty();
    let slides: Vec<String> = document
        .content
        .split("\n--\n")
        .map(|chunk| {
            let parser = MdParser::new_ext(chunk, options);

            let mut html_output = String::new();
            html::push_html(&mut html_output, parser);
            html_output
        })
        .collect();

    info!("Rendering slides");
    let content = Context {
        metadata: document.metadata,
        slides,
    };
    let rendered = TEMPLATE.render("slides.html", &tera::Context::from_serialize(&content)?)?;

    info!("Writing {}/slides.html", output_path);
    let mut out_html = output_path.to_path_buf();
    out_html.push("slides.html");
    let mut fp = std::fs::File::create(out_html)?;
    fp.write_all(rendered.as_bytes())?;
    fp.flush()?;

    info!("Done!");
    Ok(())
}

fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}

static RAW_HTML: &str = r#"""
<html>
<head>
<meta charset="utf-8"/>
<title>{{ metadata.title }}</title>
<style>
h1,h2, h3 { font-family:helvetica;font-weight:normal;margin-top:20px; }
h1 { font-size:19pt;}
h2 { font-size:14pt;margin-top:40px;}
h3 { font-size:13pt;margin-top:30px;}

td { vertical-align:top;font-size:small;padding:20px; }
body { font-family:verdana;font-size:small;text-align:center;line-height:140%;
     }
#main { text-align:left;width:900px;margin:auto; }
img { border:1px solid #8ac;width:440px;height:330px; }
li { margin-bottom:5px }
blockquote { font-style:italic; }
hr { border:1px solid #ddd; }
</style>

</head>
<body>
<div id="main"> <p><a href="https://luizirber.org/">Luiz Irber</a> > <a href="https://luizirber.org/talks/">Talks</a> > {{ metadata.title }}

<div style="background:#ffe;padding:4px;padding-left:12px;border:1px solid #aaa;margin-bottom:30px;">
  <p>This is the text version of a talk I gave on {{ metadata.date | date(format="%Y-%m-%d") }},
      at the <a href="{{ metadata.link }}">{{ metadata.event_name }}</a> conference in {{ metadata.location }}.
</div>

<h1>{{ metadata.title }}</h1>

<table>

{% for annotation in slides %}
<tr>
  <td>
    <img src="thumbs/{{ loop.index0 }}.jpg"></td>
  <td>
      {{ annotation }}
  </td>
</tr>
{% endfor %}

</table>
&copy;2022 <a href="https://luizirber.org">Luiz Irber</a>
</div>

</body>
</html>
"""#;
