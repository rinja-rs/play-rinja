use std::rc::Rc;
use std::time::Duration;

use prettyplease::unparse;
use proc_macro2::TokenStream;
use rinja_derive_standalone::derive_template;
use syn::{parse2, parse_quote};
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::js_sys::{Function, JSON};
use web_sys::wasm_bindgen::prelude::Closure;
use web_sys::wasm_bindgen::{JsCast, JsValue};
use web_sys::{window, HtmlSelectElement, Storage};
use yew::{
    function_component, html, use_effect_with, use_state, Callback, Event, Html, Properties,
    SubmitEvent, UseStateHandle,
};

use crate::editor::Editor;
use crate::{ThrowAt, ASSETS};

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    theme: Rc<str>,
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

const THEME_SOURCE_KEY: &str = "play-rinja-theme";
const STRUCT_SOURCE_KEY: &str = "play-rinja-struct";
const TMPL_SOURCE_KEY: &str = "play-rinja-template";

fn get_data_from_local_storage(storage: &Storage, key: &str) -> Option<String> {
    let text = storage.get_item(key).ok()??;
    JSON::parse(&text).ok()?.as_string()
}

#[function_component]
pub fn App() -> Html {
    let state = use_state(|| {
        let local_storage = local_storage();
        let local_storage_or = |default: &str, key: &str| -> Rc<str> {
            let value = local_storage
                .as_ref()
                .and_then(|ls| get_data_from_local_storage(ls, key));
            match value.as_deref() {
                Some(value) => value.into(),
                None => default.into(),
            }
        };

        let theme = local_storage_or(DEFAULT_THEME, THEME_SOURCE_KEY);
        let rust = local_storage_or(STRUCT_SOURCE, STRUCT_SOURCE_KEY);
        let tmpl = local_storage_or(TMPL_SOURCE, TMPL_SOURCE_KEY);

        let (code, duration) = convert_source(&rust, &tmpl);
        Props {
            theme,
            rust,
            tmpl,
            code: Rc::from(code),
            duration,
            timeout: None,
        }
    });

    let read_hash = use_state(|| false);
    if !*read_hash {
        read_hash.set(true);

        let state = state.clone();
        let theme = Rc::clone(&state.theme);
        let handler =
            Closure::<dyn Fn(Option<Vec<String>>)>::new(move |value: Option<Vec<String>>| {
                let Some(value) = value else {
                    return;
                };
                let value: Result<[String; 2], _> = value.try_into();
                let Ok([rust, tmpl]) = value else {
                    return;
                };
                if state.rust.as_ref() == rust && state.tmpl.as_ref() == tmpl {
                    return;
                }

                let (code, duration) = convert_source(&rust, &tmpl);
                state.set(Props {
                    theme: Rc::clone(&theme),
                    rust: rust.into(),
                    tmpl: tmpl.into(),
                    code: code.into(),
                    duration,
                    timeout: None,
                });
            });
        rinja_read_hash(handler.into_js_value().unchecked_ref());
    }
    use_effect_with((), move |_| {
        let handler: Closure<dyn Fn()> = Closure::new(move || read_hash.set(false));
        let _ = window().unwrap_at().add_event_listener_with_callback(
            "hashchange",
            handler.into_js_value().unchecked_ref(),
        );
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
                if let Ok(data) = JSON::stringify(&JsValue::from_str(&data)) {
                    if let Some(data) = data.as_string() {
                        // Doesn't matter whether or not it succeeded.
                        let _ = storage.set_item(storage_name, &data);
                    }
                }
            }

            let mut new_state = Props::clone(&*state);
            edit(&mut new_state, data);
            replace_timeout(&mut new_state, state.clone());
            state.set(new_state);
        }
    };
    let oninput_rust = oninput(STRUCT_SOURCE_KEY, |new_state, data| {
        new_state.rust = Rc::from(data);
    });
    let oninput_tmpl = oninput(TMPL_SOURCE_KEY, |new_state, data| {
        new_state.tmpl = Rc::from(data);
    });

    let onchange_theme = {
        let state = state.clone();
        move |ev: Event| {
            let Some(target) = ev.target() else {
                return;
            };
            let target: HtmlSelectElement = target.unchecked_into();
            let data = target.value();

            if let Some(storage) = local_storage() {
                // Doesn't matter whether or not it succeeded.
                let _ = storage.set_item(THEME_SOURCE_KEY, &data);
            }
            state.set(Props {
                theme: data.into(),
                ..Props::clone(&state)
            })
        }
    };

    let theme = state.theme.as_ref();
    let (_, themes) = *ASSETS;
    let (theme_idx, theme) = match themes
        .iter()
        .enumerate()
        .find_map(|(idx, &(key, value))| (key == theme).then_some((idx, value)))
    {
        Some((theme_idx, theme)) => (theme_idx, theme),
        None => {
            state.set(Props {
                theme: DEFAULT_THEME.into(),
                ..Props::clone(&state)
            });
            (0, themes[0].1) // index does not matter, will be rerendered immediately
        }
    };

    let themes = themes
        .iter()
        .copied()
        .enumerate()
        .map(|(i, (value, _))| {
            html! {
                <option value={value} selected={i == theme_idx}>
                    {value}
                </option>
            }
        })
        .collect::<Html>();

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
        let theme = Rc::clone(&new_state.theme);
        let rust = Rc::clone(&new_state.rust);
        let tmpl = Rc::clone(&new_state.tmpl);
        let state = state.clone();
        move || {
            let (code, duration) = convert_source(&rust, &tmpl);
            state.set(Props {
                theme: Rc::clone(&theme),
                rust: Rc::clone(&rust),
                tmpl: Rc::clone(&tmpl),
                code: Rc::from(code),
                duration,
                timeout: None,
            });

            rinja_update_hash(&rust, &tmpl);
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

#[wasm_bindgen]
extern "C" {
    fn rinja_update_hash(rust: &str, tmpl: &str);
    fn rinja_read_hash(callback: &Function);
}
