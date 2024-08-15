use std::rc::Rc;
use std::time::Duration;

use prettyplease::unparse;
use proc_macro2::TokenStream;
use rinja_derive_standalone::derive_template;
use syn::{parse2, parse_quote};
use web_sys::wasm_bindgen::prelude::Closure;
use web_sys::wasm_bindgen::JsCast;
use web_sys::{window, HtmlSelectElement, Storage};
use yew::{
    function_component, html, use_state, Callback, Event, Html, Properties, SubmitEvent,
    UseStateHandle,
};

use crate::editor::Editor;
use crate::{ThrowAt, ASSETS};

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    theme: usize,
    rust: Rc<str>,
    tmpl: Rc<str>,
    code: Rc<str>,
    duration: Option<Duration>,
    timeout: Option<i32>,
}

fn local_storage() -> Option<Storage> {
    let window = window()?;
    match window.local_storage() {
        Ok(storage) => storage,
        _ => None,
    }
}

fn get_data_from_local_storage(storage: &Storage, key: &str) -> Option<String> {
    let source = storage.get_item(key).ok()??;
    match json::parse(&source) {
        Ok(json::JsonValue::String(source)) => Some(source),
        _ => None,
    }
}

#[function_component]
pub fn App() -> Html {
    let state = use_state(|| {
        let tmp;
        let tmp2;
        let (struct_source, template_source) = if let Some(storage) = local_storage() {
            let struct_source =
                match get_data_from_local_storage(&storage, "rinja-app-struct-source") {
                    Some(source) => {
                        tmp = source;
                        &tmp
                    }
                    _ => STRUCT_SOURCE,
                };
            let template_source =
                match get_data_from_local_storage(&storage, "rinja-app-template-source") {
                    Some(source) => {
                        tmp2 = source;
                        &tmp2
                    }
                    _ => TMPL_SOURCE,
                };
            (struct_source, template_source)
        } else {
            (STRUCT_SOURCE, TMPL_SOURCE)
        };
        let (code, duration) = convert_source(struct_source, template_source);

        let theme = ASSETS
            .1
            .iter()
            .position(|&(theme, _)| theme == DEFAULT_THEME)
            .unwrap_or_default();

        Props {
            theme,
            rust: Rc::from(struct_source),
            tmpl: Rc::from(template_source),
            code: Rc::from(code),
            duration,
            timeout: None,
        }
    });

    let duration = state.duration.map(|d| format!(" (duration: {d:?})"));

    let onsubmit = Callback::from(|ev: SubmitEvent| {
        ev.prevent_default();
        ev.stop_propagation();
    });

    let oninput = |storage_name: &'static str, edit: fn(&mut Props, String)| {
        let state = state.clone();
        move |data: String| {
            if let Some(storage) = local_storage() {
                // Doesn't matter whether or not it succeeded.
                let _ = storage.set_item(storage_name, &json::stringify(data.as_str()));
            }

            let mut new_state = Props::clone(&*state);
            edit(&mut new_state, data);
            replace_timeout(&mut new_state, state.clone());
            state.set(new_state);
        }
    };
    let oninput_rust = oninput("rinja-app-struct-source", |new_state, data| {
        new_state.rust = Rc::from(data)
    });
    let oninput_tmpl = oninput("rinja-app-template-source", |new_state, data| {
        new_state.tmpl = Rc::from(data)
    });

    let theme_idx = state.theme;
    let (_, themes) = *ASSETS;
    let (_, theme) = themes[theme_idx];

    let themes = themes
        .iter()
        .copied()
        .enumerate()
        .map(|(i, (value, _))| {
            html! {
                <option
                    value={i.to_string()}
                    selected={i == theme_idx}
                >
                    {value}
                </option>
            }
        })
        .collect::<Html>();

    let onchange_theme = Callback::from({
        let state = state.clone();
        move |ev: Event| {
            let Some(target) = ev.target() else {
                return;
            };
            let target: HtmlSelectElement = target.unchecked_into();
            let Ok(theme) = target.selected_index().try_into() else {
                return;
            };

            let old_state = &*state;
            state.set(Props {
                theme,
                rust: Rc::clone(&old_state.rust),
                tmpl: Rc::clone(&old_state.tmpl),
                code: Rc::clone(&old_state.code),
                duration: old_state.duration,
                timeout: old_state.timeout,
            });
        }
    });

    html! {
        <form method="GET" action="javascript:;" {onsubmit}>
            <div id="top">
                <div>
                    <h3> {"Your struct:"} </h3>
                    <Editor
                        text={Rc::clone(&state.rust)}
                        oninput={oninput_rust}
                        syntax="Rust"
                        {theme}
                    />
                </div>
                <div>
                    <h3> {"Your template:"} </h3>
                    <Editor
                        text={Rc::clone(&state.tmpl)}
                        oninput={oninput_tmpl}
                        syntax="HTML (Jinja2)"
                        {theme}
                    />
                </div>
            </div>
            <div>
                <h3> {"Generated code:"} {duration} </h3>
                <Editor
                    text={Rc::clone(&state.code)}
                    syntax="Rust"
                        {theme}
                />
            </div>
            <div id="rev">
                <a href={TREE_URL} target="_blank" rel="noopener">
                    <abbr title="Rinja revision">
                        {env!("RINJA_DESCR")}
                    </abbr>
                </a>
            </div>
            <div>
                <label>
                    <strong> {"Theme: "} </strong>
                    <select onchange={onchange_theme}> {themes} </select>
                </label>
            </div>
            <div id="bottom">
                <a href="https://crates.io/crates/rinja" title="Crates.io">
                    <img
                        src="https://img.shields.io/crates/v/rinja?logo=rust&style=flat-square&logoColor=white"
                        alt="Crates.io"
                    />
                </a>
                {" "}
                <a
                    href="https://github.com/rinja-rs/rinja/actions/workflows/rust.yml"
                    title="GitHub Workflow Status"
                >
                    <img
                        src="https://img.shields.io/github/actions/workflow/status/rinja-rs/rinja/rust.yml?\
                             branch=master&logo=github&style=flat-square&logoColor=white"
                        alt="GitHub Workflow Status"
                    />
                </a>
                {" "}
                <a href="https://rinja.readthedocs.io/" title="Book">
                    <img
                        src="https://img.shields.io/readthedocs/rinja?label=book&logo=readthedocs&style=flat-square&logoColor=white"
                        alt="Book"
                    />
                </a>
                {" "}
                <a href="https://docs.rs/rinja/" title="docs.rs">
                    <img
                        src="https://img.shields.io/docsrs/rinja?logo=docsdotrs&style=flat-square&logoColor=white"
                        alt="docs.rs"
                    />
                </a>
            </div>
            <div id="fork">
                <a href="https://github.com/rinja-rs/play-rinja" title="Fork me on GitHub">
                    <svg viewBox="0 0 250 250" aria-hidden="true">
                        <path d="M0,0 L115,115 L130,115 L142,142 L250,250 L250,0 Z" />
                        <path fill="currentColor" class="octo-arm" d="M128.3,109.0 C113.8,99.7 119.0,89.6 119.0,89.6 C122.0,82.7 120.5,78.6 120.5,78.6 C119.2,72.0 123.4,76.3 123.4,76.3 C127.3,80.9 125.5,87.3 125.5,87.3 C122.9,97.6 130.6,101.9 134.4,103.2" />
                        <path fill="currentColor" class="octo-body" d="M115.0,115.0 C114.9,115.1 118.7,116.5 119.8,115.4 L133.7,101.6 C136.9,99.2 139.9,98.4 142.2,98.6 C133.8,88.0 127.5,74.4 143.8,58.0 C148.5,53.4 154.0,51.2 159.7,51.0 C160.3,49.4 163.2,43.6 171.4,40.1 C171.4,40.1 176.1,42.5 178.8,56.2 C183.1,58.6 187.2,61.8 190.9,65.4 C194.5,69.0 197.7,73.2 200.1,77.6 C213.8,80.2 216.3,84.9 216.3,84.9 C212.7,93.1 206.9,96.0 205.4,96.6 C205.1,102.4 203.0,107.8 198.3,112.5 C181.9,128.9 168.3,122.5 157.7,114.1 C157.9,116.9 156.7,120.9 152.7,124.9 L141.0,136.5 C139.8,137.7 141.6,141.9 141.8,141.8 Z" />
                    </svg>
                </a>
            </div>
        </form>
    }
}

fn replace_timeout(new_state: &mut Props, state: UseStateHandle<Props>) {
    let handler = Closure::<dyn Fn()>::new({
        let theme = new_state.theme;
        let rust = Rc::clone(&new_state.rust);
        let tmpl = Rc::clone(&new_state.tmpl);
        let state = state.clone();
        move || {
            let (code, duration) = convert_source(&rust, &tmpl);
            state.set(Props {
                theme,
                rust: Rc::clone(&rust),
                tmpl: Rc::clone(&tmpl),
                code: Rc::from(code),
                duration,
                timeout: None,
            });
        }
    });

    let window = window().unwrap_at();
    if let Some(timeout) = new_state.timeout {
        window.clear_timeout_with_handle(timeout);
    }
    new_state.timeout = window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            handler.into_js_value().unchecked_ref(),
            500,
        )
        .ok();
}

fn convert_source(rust: &str, tmpl: &str) -> (String, Option<Duration>) {
    let mut code: TokenStream = parse_quote! { #[template(source = #tmpl)] };
    code.extend(rust.parse::<TokenStream>());
    let (code, duration) = time_it(|| derive_template(code));
    let mut code = unparse(&parse2(code).unwrap_at());
    code.truncate(code.trim_end().len());
    (code, duration)
}

fn time_it<F: FnOnce() -> R, R>(func: F) -> (R, Option<Duration>) {
    let performance = window().unwrap_at().performance();
    let start = performance.as_ref().map(|p| p.now());
    let result = func();
    let end = performance.as_ref().map(|p| p.now());
    let duration = match (start, end) {
        (Some(start), Some(end)) => Duration::try_from_secs_f64((end - start) / 1000.0).ok(),
        _ => None,
    };
    (result, duration)
}

const DEFAULT_THEME: &str = "Monokai Extended Origin";

const TREE_URL: &str = concat!(env!("RINJA_URL"), "/tree/", env!("RINJA_REV"));

const TMPL_SOURCE: &str = r##"<div class="example">
    Hello, <strong>{{user}}</strong>!
    {%~ if first_visit -%}
        <br />
        Nice to meet you.
    {%~ endif -%}
</div>"##;

const STRUCT_SOURCE: &str = r##"#[derive(Template)]
#[template(ext = "html")]
// in the preview, the `source="…"` or `path="…"` argument is provided for you
struct HelloWorld<'a> {
    user: &'a str,
    first_visit: bool,
}"##;
