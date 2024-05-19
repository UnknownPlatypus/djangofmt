use std::process::ExitCode;

use clap::Parser;
use colored::Colorize;
use markup_fmt::{format_text, Language};
use markup_fmt::config::{FormatOptions, LanguageOptions, LayoutOptions};

use djangofmt::{Args, ExitStatus, run};

pub fn main() -> ExitCode {
    let args = std::env::args_os();

    let args = Args::parse_from(args);

    match run(args) {
        Ok(code) => code.into(),
        Err(err) => {
            #[allow(clippy::print_stderr)]
            {
                // Unhandled error from djangofmt.
                eprintln!("{}", "djangofmt failed".red().bold());
                for cause in err.chain() {
                    eprintln!("  {} {cause}", "Cause:".bold());
                }
            }
            ExitStatus::Error.into()
        }
    }
}

fn test() {
    let options = FormatOptions {
        layout: LayoutOptions {
            print_width: 120,
            indent_width: 4,
            ..LayoutOptions::default()
        },
        language: LanguageOptions {
            closing_bracket_same_line: false, // This is default, remove later
            ..LanguageOptions::default()
        },
    };

    let html = r#"
    {% load comparator_extras %}
    {% load i18n %}
    <html>
        <head>
         <title>Example</title>
         <style>button { outline: none; }</style>
        </head>
        
        <body>
        <div class="card_rating_container">
            {% if provider.reviews_count > 0 %}
                <a href="{% url 'comparator:reviews-provider' provider_name=provider.url_name %}"
                   class="card_link">
                    <span class="">{% show_stars provider.rating %}</span>
                    <div class="card_rating_number has_rating">({{ provider.reviews_count }} {% translate "avis" %})</div>
                </a>
            {% elif offer.provider.reviews_count > 0 %}
                <a href="{% url 'comparator:reviews-provider' provider_name=offer.provider.url_name %}"
                   class="card_link">
                    <span class="">{% show_stars offer.provider.rating %}</span>
                    <div class="card_rating_number has_rating">({{ offer.provider.reviews_count }} {% translate "avis" %})</div>
                </a>
            {% else %}
                <span class="odp-main-stars-none">{% show_stars 5 %}</span>
                <div class="card_rating_number hw-grey-4">{% translate "(Aucun avis)" %}</div>
            {% endif %}
        </div>

        <script>const a = 1;</script>
        </body>
    </html>"#;

    println!("============================================================");
    println!(
        "{}",
        format_text(html, Language::Jinja, &options, |_, code, _| Ok::<_, ()>(
            code.into()
        ),)
        .unwrap()
    );
    println!("============================================================");
}
