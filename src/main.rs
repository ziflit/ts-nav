use anstyle::{Color, RgbColor, Style};
use ignore::WalkBuilder;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt::Write;
use std::{env, fs};
use tree_sitter::Language;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};
use tree_sitter_tags::{TagsConfiguration, TagsContext};

struct LangConfig {
    pub language: Language,
    pub highlights: String,
    pub query: String,
}

fn highlight(input: &str, config: &LangConfig) -> String {
    let theme: HashMap<&str, Color> = [
        ("attribute", Color::Rgb(RgbColor::from((187, 194, 207)))),
        ("constant", Color::Rgb(RgbColor::from((255, 135, 0)))),
        ("function.builtin", Color::Rgb(RgbColor::from((247, 187, 59)))),
        ("function", Color::Rgb(RgbColor::from((247, 187, 59)))),
        ("keyword", Color::Rgb(RgbColor::from((175, 215, 0)))),
        ("operator", Color::Rgb(RgbColor::from((209, 109, 158)))),
        ("property", Color::Rgb(RgbColor::from((198, 120, 221)))),
        ("punctuation", Color::Rgb(RgbColor::from((128, 160, 194)))),
        ("punctuation.bracket", Color::Rgb(RgbColor::from((128, 160, 194)))),
        ("punctuation.delimiter", Color::Rgb(RgbColor::from((128, 160, 194)))),
        ("punctuation.special", Color::Rgb(RgbColor::from((128, 160, 194)))),
        ("string", Color::Rgb(RgbColor::from((250, 183, 149)))),
        ("string.special", Color::Rgb(RgbColor::from((250, 183, 149)))),
        ("tag", Color::Rgb(RgbColor::from((255, 135, 0)))),
        ("type", Color::Rgb(RgbColor::from((233, 86, 120)))),
        ("type.builtin", Color::Rgb(RgbColor::from((26, 188, 156)))),
        ("variable", Color::Rgb(RgbColor::from((242, 242, 191)))),
        ("variable.builtin", Color::Rgb(RgbColor::from((242, 242, 191)))),
        ("variable.parameter", Color::Rgb(RgbColor::from((97, 175, 239)))),
    ]
    .iter()
    .cloned()
    .collect();
    let highlight_names: Vec<String> = theme.iter().map(|(&k, _)| String::from(k)).collect();
    let styles: Vec<anstyle::Style> = highlight_names
        .iter()
        .map(|hg| {
            let style = anstyle::Style::new();
            let fg_color = Some(theme[hg.as_str()]);
            style.fg_color(fg_color)
        })
        .collect();

    let mut highlighter = Highlighter::new();
    let mut python_config = HighlightConfiguration::new(
        config.language.to_owned(),
        "whatever",
        &config.highlights,
        "",
        "",
    )
    .unwrap();
    python_config.configure(&highlight_names);
    let highlights = highlighter
        .highlight(&python_config, input.as_bytes(), None, |_| None)
        .unwrap();

    let mut style_stack = vec![Style::new()];
    let mut out: String = String::new();
    for event in highlights {
        match event.unwrap() {
            HighlightEvent::Source { start, end } => {
                let style: &Style = &style_stack.last().unwrap();
                write!(&mut out, "{style}").unwrap();
                write!(&mut out, "{}", &input[start..end]).unwrap();
                write!(&mut out, "{style:#}").unwrap();
            }
            HighlightEvent::HighlightStart(highlight) => {
                style_stack.push(styles[highlight.0]);
            }
            HighlightEvent::HighlightEnd => {
                style_stack.pop();
            }
        }
    }
    out
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let query_str = &args[1];
    let path = if args.len() > 2 { &args[2] } else { "." };

    let python_function_definition_query = format!(
        r#"
        ; defs
        ((function_definition
             name: (identifier) @name
             parameters: (parameters)
             return_type: (type)?) @definition.function (#match? @name "(?i).*{query_str}.*"))

        ; refs
        ((call
          function: [
              (identifier) @name
              (attribute
                attribute: (identifier) @name)
          ]) @reference.call
         (#match? @name "(?i).*{query_str}.*"))
    "#
    );
    let rust_function_definition_query = format!(
        r#"
        ; defs
        ((function_item
            name: (identifier) @name) @definition.function
        (#match? @name "(?i).*{query_str}.*"))

        ((declaration_list
            (function_item
                name: (identifier) @name)) @definition.method
        (#match? @name "(?i).*{query_str}.*"))
        ; refs

        ((call_expression function: (scoped_identifier) @name) @reference.call
         (#match? @name "(?i).*{query_str}.*"))

        ((call_expression
            function: (identifier) @name) @reference.call
        (#match? @name "(?i).*{query_str}.*"))

        ; method
        ((call_expression
            function: (field_expression
                field: (field_identifier) @name)) @reference.call
        (#match? @name "(?i).*{query_str}.*"))
    "#
    );

    let mut context = TagsContext::new();

    let walker = WalkBuilder::new(path)
        .add_custom_ignore_filename(".gitignore")
        .build();

    for some_entry in walker {
        let Ok(entry) = some_entry else { continue };
        // TODO ta mal esto
        let lang_config = match entry.path().extension().unwrap_or(&OsStr::new("")).to_str() {
            Some("py") => LangConfig {
                language: tree_sitter_python::LANGUAGE.into(),
                highlights: String::from(tree_sitter_python::HIGHLIGHTS_QUERY),
                query: String::from(&python_function_definition_query),
            },
            Some("rs") => LangConfig {
                language: tree_sitter_rust::LANGUAGE.into(),
                highlights: String::from(tree_sitter_rust::HIGHLIGHTS_QUERY),
                query: String::from(&rust_function_definition_query),
            },
            Some(_) => continue,
            None => continue,
        };
        let code = fs::read_to_string(entry.path()).unwrap();

        let tag_config =
            TagsConfiguration::new(lang_config.language.to_owned(), &lang_config.query, "");
        match tag_config {
            Ok(t) => {
                let (tags, _) = context.generate_tags(&t, code.as_bytes(), None).unwrap();
                for tag in tags {
                    match tag {
                        Ok(t) => {
                            println!(
                                "{}:{}:{}",
                                entry.path().to_str().unwrap(),
                                t.span.start.row + 1,
                                t.span.start.column + 1
                            );
                            println!("{}", highlight(&code[t.range], &lang_config));
                        }
                        Err(err) => {
                            println!("Error in query: {}", err)
                        }
                    }
                }
            }
            Err(err) => {
                println!("Error in config: {}", err);
            }
        }
    }
}
