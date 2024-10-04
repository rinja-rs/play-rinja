use std::fmt::Write;
use std::rc::Rc;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, FontStyle, Theme};
use syntect::util::LinesWithEndings;
use web_sys::HtmlTextAreaElement;
use web_sys::wasm_bindgen::JsCast;
use yew::{Callback, Html, InputEvent, Properties, function_component, html};

use crate::{ASSETS, ThrowAt};

#[derive(Properties, PartialEq)]
pub struct EditorProps {
    pub text: Rc<str>,
    pub syntax: &'static str,
    pub theme: &'static Theme,
    #[prop_or_default]
    pub oninput: Option<Callback<String>>,
    #[prop_or_default]
    pub id: Option<&'static str>,
}

#[function_component]
pub fn Editor(props: &EditorProps) -> Html {
    let EditorProps {
        text,
        syntax,
        theme,
        oninput,
        id,
    } = props;
    html! {
        <div class="editor">
            <UnstylizedCode text={Rc::clone(text)} theme={*theme} {oninput} {id} />
            <StylizedCode text={Rc::clone(text)} syntax={*syntax} theme={*theme} />
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct UnstylizedCodeProps {
    pub text: Rc<str>,
    pub theme: &'static Theme,
    #[prop_or_default]
    pub oninput: Option<Callback<String>>,
    #[prop_or_default]
    pub id: Option<&'static str>,
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

    let settings = &props.theme.settings;
    let caret = settings
        .caret
        .or(settings.foreground)
        .unwrap_or(Color::BLACK);

    html! {
        <textarea
            autocapitalize="off"
            spellcheck="false"
            readonly={oninput.is_none()}
            value={Rc::clone(&props.text)}
            style={format!("caret-color:#{:02x}{:02x}{:02x};", caret.r, caret.b, caret.b)}
            id={props.id}
            {oninput}
        />
    }
}

#[derive(Properties, PartialEq)]
pub struct StylizedCodeProps {
    pub text: Rc<str>,
    pub syntax: &'static str,
    pub theme: &'static Theme,
}

#[function_component]
pub fn StylizedCode(props: &StylizedCodeProps) -> Html {
    let (syntax_set, _) = *ASSETS;
    let syntax = syntax_set.find_syntax_by_name(props.syntax).unwrap_at();

    let theme = props.theme;
    let fg = theme.settings.foreground.unwrap_or(Color::BLACK);
    let bg = theme.settings.background.unwrap_or(Color::WHITE);
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut output = Vec::new();
    let mut cur_style = String::new();
    let mut accu_string = String::new();

    for line in LinesWithEndings::from(&props.text) {
        let regions = highlighter.highlight_line(line, syntax_set).unwrap_at();
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

            if style == cur_style {
                accu_string.push_str(text);
            } else {
                if !accu_string.is_empty() {
                    if cur_style.is_empty() {
                        output.push(html!({ accu_string.to_owned() }));
                    } else {
                        output.push(html! {
                            <span style={cur_style.to_owned()}> {accu_string.to_owned()} </span>
                        });
                    }
                    accu_string.clear();
                }
                accu_string.push_str(text);
                cur_style = style;
            }
        }
    }
    if !accu_string.is_empty() {
        if cur_style.is_empty() {
            output.push(html!({ accu_string }));
        } else {
            output.push(html!(<span style={cur_style}>{accu_string}</span>));
        }
    }

    let style = format!(
        "color:#{:02x}{:02x}{:02x};background-color:#{:02x}{:02x}{:02x};",
        fg.r, fg.g, fg.b, bg.r, bg.g, bg.b,
    );

    html! {
        <pre {style}>
            {"\u{feff}"} {output} {"\u{feff}"}
        </pre>
    }
}

fn write_css_color(s: &mut String, c: Color) {
    if c.a != 0xFF {
        write!(s, "#{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, c.a).unwrap_at();
    } else {
        write!(s, "#{:02x}{:02x}{:02x}", c.r, c.g, c.b).unwrap_at();
    }
}
