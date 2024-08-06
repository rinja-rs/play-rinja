use std::fmt::Write;
use std::rc::Rc;

use once_cell::sync::Lazy;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use web_sys::wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::HtmlTextAreaElement;
use yew::{function_component, html, Callback, Html, InputEvent, Properties};

#[derive(Properties, PartialEq)]
pub struct EditorProps {
    pub text: Rc<str>,
    #[prop_or_default]
    pub oninput: Callback<String>,
    pub syntax: &'static str,
}

#[function_component]
pub fn Editor(props: &EditorProps) -> Html {
    let EditorProps {
        text,
        oninput,
        syntax,
    } = props;
    html! {
        <div class="editor">
            <UnstylizedCode text={Rc::clone(text)} {oninput} />
            <StylizedCode text={Rc::clone(text)} syntax={*syntax} />
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct UnstylizedCodeProps {
    pub text: Rc<str>,
    #[prop_or_default]
    pub oninput: Option<Callback<String>>,
}

#[function_component]
pub fn UnstylizedCode(props: &UnstylizedCodeProps) -> Html {
    let oninput = props.oninput.clone().map(|oninput| {
        Callback::from({
            let old_text = Rc::clone(&props.text);
            move |ev: InputEvent| {
                let Some(target) = ev.target() else { return };
                let target: HtmlTextAreaElement = target.unchecked_into();
                let new_text = target.value();
                if new_text != *old_text {
                    oninput.emit(new_text);
                }
            }
        })
    });

    html! {
        <textarea
            autocapitalize="off"
            spellcheck="false"
            readonly={oninput.is_none()}
            value={Rc::clone(&props.text)}
            {oninput}
        />
    }
}

#[function_component]
pub fn StylizedCode(props: &StylizedCodeProps) -> Html {
    let syntax = SYNTAX_SET.find_syntax_by_name(props.syntax).unwrap_throw();
    let theme = &THEME_SET.themes["InspiredGitHub"];

    let fg = theme.settings.foreground.unwrap_or(Color::BLACK);
    let bg = theme.settings.background.unwrap_or(Color::WHITE);
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut output = Vec::new();
    let mut last_style: Option<Rc<str>> = None;
    let mut last_string = String::new();

    for line in LinesWithEndings::from(&props.text) {
        let regions = highlighter.highlight_line(line, &SYNTAX_SET).unwrap_throw();
        for (s, text) in regions {
            let mut style = String::new();

            if s.foreground != fg {
                style.push_str("color:");
                write_css_color(&mut style, s.foreground);
                style.push(';');
            }
            if s.background != bg {
                style.push_str("background-color:");
                write_css_color(&mut style, s.background);
                style.push(';');
            }
            if s.font_style.contains(FontStyle::BOLD) {
                style.push_str("font-weight:bold;");
            }
            if s.font_style.contains(FontStyle::UNDERLINE) {
                style.push_str("text-decoration:underline;");
            }
            if s.font_style.contains(FontStyle::ITALIC) {
                style.push_str("font-style:italic;");
            }

            let style = match style.is_empty() {
                true => None,
                false => Some(style),
            };
            if style.as_deref() != last_style.as_deref() {
                last_style = style.map(Rc::from);
            }

            if let Some(last_style) = last_style.clone() {
                if !last_string.is_empty() {
                    output.push(html!({ String::from(&last_string) }));
                    last_string.clear();
                }
                output.push(html!(<span style={last_style}>{text}</span>));
            } else {
                last_string.push_str(text);
            }
        }
    }
    if !last_string.is_empty() {
        output.push(html!({ last_string }));
    }

    let style = format!(
        "color:#{:02x}{:02x}{:02x};background-color:#{:02x}{:02x}{:02x};",
        fg.r, fg.g, fg.b, bg.r, bg.g, bg.b,
    );

    html! {
        <pre {style}>
            {output}
            {"\u{feff}"}
        </pre>
    }
}

#[derive(Properties, PartialEq)]
pub struct StylizedCodeProps {
    pub text: Rc<str>,
    pub syntax: &'static str,
}

fn write_css_color(s: &mut String, c: Color) {
    if c.a != 0xFF {
        write!(s, "#{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, c.a).unwrap_throw();
    } else {
        write!(s, "#{:02x}{:02x}{:02x}", c.r, c.g, c.b).unwrap_throw();
    }
}

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);
