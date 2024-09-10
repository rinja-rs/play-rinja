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
use web_sys::{window, FocusEvent, HtmlDialogElement, HtmlSelectElement, Storage};
use yew::{
    function_component, html, use_effect_with, use_state, Callback, Event, Html, MouseEvent,
    Properties, SubmitEvent, UseStateHandle,
};

use crate::editor::Editor;
use crate::{ThrowAt, ASSETS};

#[derive(Properties, PartialEq, Clone)]
struct Props {
    theme: Rc<str>,
    rust: Rc<str>,
    tmpl: Rc<str>,
    code: Rc<str>,
    duration: Option<Duration>,
    timeout: Option<i32>,
}

#[function_component]
pub fn App() -> Html {
    let state = use_state(|| {
        let (theme, rust, tmpl) = get_last_editor_state().unwrap_or_default();
        let theme = theme.unwrap_or_else(|| Rc::from(DEFAULT_THEME));
        let rust = rust.unwrap_or_else(|| Rc::from(STRUCT_SOURCE));
        let tmpl = tmpl.unwrap_or_else(|| Rc::from(TMPL_SOURCE));
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

    // share_dialog
    let (saved_url, saved_url_open, saved_url_onclose, saved_url_close, saved_url_copy);
    #[allow(clippy::let_unit_value)]
    let _ = {
        use_effect_with((), {
            let state = state.clone();
            move |_| {
                let callback: Closure<dyn Fn(Option<String>, Option<String>)> =
                    Closure::new(move |rust: Option<String>, tmpl: Option<String>| {
                        let (Some(rust), Some(tmpl)) = (rust, tmpl) else {
                            return;
                        };
                        if let Some(storage) = local_storage() {
                            // Doesn't matter whether or not it succeeded.
                            let _ = save_to_local_storage(&storage, STRUCT_SOURCE_KEY, &rust);
                            let _ = save_to_local_storage(&storage, TMPL_SOURCE_KEY, &tmpl);
                        }
                        let (code, duration) = convert_source(&rust, &tmpl);
                        state.set(Props {
                            theme: Rc::clone(&state.theme),
                            rust: Rc::from(rust),
                            tmpl: Rc::from(tmpl),
                            code: Rc::from(code),
                            duration,
                            timeout: None,
                        });
                    });
                read_saved_url(callback.into_js_value().unchecked_ref());
            }
        });

        saved_url = use_state(|| Option::<Rc<str>>::None);

        saved_url_open = {
            let saved_url = saved_url.clone();
            move |_: MouseEvent| {
                let saved_url = saved_url.clone();
                let callback: Closure<dyn Fn(Option<String>)> =
                    Closure::new(move |url: Option<String>| {
                        saved_url.set(url.map(Rc::from));
                    });
                gen_saved_url(callback.into_js_value().unchecked_ref());
            }
        };

        saved_url_onclose = {
            let saved_url = saved_url.clone();
            move |_: Event| saved_url.set(None)
        };

        saved_url_close = move |_: MouseEvent| {
            if let Some(share_dialog) = share_dialog() {
                let _ = share_dialog.close();
            }
        };

        saved_url_copy = saved_url
            .as_ref()
            .map(Rc::clone)
            .map(|saved_url| move |_: MouseEvent| save_clipboard(&saved_url));

        if saved_url.is_some() {
            if let Some(share_dialog) = share_dialog() {
                let share_dialog: HtmlDialogElement = share_dialog.unchecked_into();
                let _ = share_dialog.show_modal();
            }
        }
    };

    let onsubmit = Callback::from(|ev: SubmitEvent| {
        ev.prevent_default();
        ev.stop_propagation();
    });

    let oninput = |storage_name: &'static str, edit: fn(&mut Props, String)| {
        let state = state.clone();
        move |data: String| {
            if let Some(storage) = local_storage() {
                save_to_local_storage(&storage, storage_name, &data);
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
                save_to_local_storage(&storage, THEME_SOURCE_KEY, &data);
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
        <div>
            <header>
                <button
                    id="settings-menu"
                    type="button"
                    class="dropdown-menu"
                    onclick={|event: MouseEvent| toggle_element(event, "settings-menu")}
                    onblur={|event: FocusEvent| handle_blur(event, "settings-menu")}
                >
                    {"Settings"}
                    <div tabindex="-1" onblur={|event: FocusEvent| handle_blur(event, "settings-menu")}>
                        <label>
                            <strong>{"Theme: "}</strong>
                            <select onchange={onchange_theme} id="theme" onblur={|event: FocusEvent| handle_blur(event, "settings-menu")}>
                                {themes}
                            </select>
                        </label>
                    </div>
                </button>
                <button
                    id="info-menu"
                    type="button"
                    class="dropdown-menu"
                    onclick={|event: MouseEvent| toggle_element(event, "info-menu")}
                    onblur={|event: FocusEvent| handle_blur(event, "info-menu")}
                >
                    {"Info"}
                    <div tabindex="-1" onblur={|event: FocusEvent| handle_blur(event, "info-menu")}>
                        <a href="https://crates.io/crates/rinja" title="Crates.io">
                            <img
                                src="https://img.shields.io/crates/v/rinja?logo=rust&style=flat-square&logoColor=white"
                                alt="Crates.io"
                            />
                        </a>
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
                        <a href="https://rinja.readthedocs.io/" title="Book">
                            <img
                                src="https://img.shields.io/readthedocs/rinja?label=book&logo=readthedocs&style=flat-square&logoColor=white"
                                alt="Book"
                            />
                        </a>
                        <a href="https://docs.rs/rinja/" title="docs.rs">
                            <img
                                src="https://img.shields.io/docsrs/rinja?logo=docsdotrs&style=flat-square&logoColor=white"
                                alt="docs.rs"
                            />
                        </a>
                        <p>
                            <strong>{"Rinja Revision:"}</strong>
                            <br />
                            <a href={TREE_URL} target="_blank">
                                {env!("RINJA_DESCR")}
                            </a>
                        </p>
                    </div>
                </button>
                <button type="button" onclick={saved_url_open}>
                    {"Share"}
                </button>
                <div id="fork">
                    <a href="https://github.com/rinja-rs/play-rinja" title="Fork me on GitHub">
                        <svg viewBox="0 0 250 250" aria-hidden="true">
                            <path d="M0,0 L115,115 L130,115 L142,142 L250,250 L250,0 Z" />
                            <path fill="currentColor" class="octo-arm" d="M128.3,109.0 C113.8,99.7 119.0,89.6 119.0,89.6 C122.0,82.7 120.5,78.6 120.5,78.6 C119.2,72.0 123.4,76.3 123.4,76.3 C127.3,80.9 125.5,87.3 125.5,87.3 C122.9,97.6 130.6,101.9 134.4,103.2" />
                            <path fill="currentColor" class="octo-body" d="M115.0,115.0 C114.9,115.1 118.7,116.5 119.8,115.4 L133.7,101.6 C136.9,99.2 139.9,98.4 142.2,98.6 C133.8,88.0 127.5,74.4 143.8,58.0 C148.5,53.4 154.0,51.2 159.7,51.0 C160.3,49.4 163.2,43.6 171.4,40.1 C171.4,40.1 176.1,42.5 178.8,56.2 C183.1,58.6 187.2,61.8 190.9,65.4 C194.5,69.0 197.7,73.2 200.1,77.6 C213.8,80.2 216.3,84.9 216.3,84.9 C212.7,93.1 206.9,96.0 205.4,96.6 C205.1,102.4 203.0,107.8 198.3,112.5 C181.9,128.9 168.3,122.5 157.7,114.1 C157.9,116.9 156.7,120.9 152.7,124.9 L141.0,136.5 C139.8,137.7 141.6,141.9 141.8,141.8 Z" />
                        </svg>
                    </a>
                </div>
            </header>
            <form id="content" method="GET" action="javascript:;" {onsubmit}>
                <div id="top">
                    <div>
                        <h3>
                            <button class="reset" onclick={|event| reset_code(event, STRUCT_SOURCE)}>
                                {"Reset code"}
                            </button>
                            {"Your struct:"}
                        </h3>
                        <Editor
                            text={Rc::clone(&state.rust)}
                            oninput={oninput_rust}
                            syntax="Rust"
                            id="rust"
                            {theme}
                        />
                    </div>
                    <div>
                        <h3>
                            <button class="reset" onclick={|event| reset_code(event, TMPL_SOURCE)}>
                                {"Reset code"}
                            </button>
                            {"Your template:"}
                        </h3>
                        <Editor
                            text={Rc::clone(&state.tmpl)}
                            oninput={oninput_tmpl}
                            syntax="HTML (Jinja2)"
                            id="tmpl"
                            {theme}
                        />
                    </div>
                </div>
                <div>
                    <h3>
                        {"Generated code:"}
                        {state.duration.map(|d| format!(" (duration: {d:?})"))}
                    </h3>
                    <Editor
                        text={Rc::clone(&state.code)}
                        syntax="Rust"
                        id="code"
                        {theme}
                    />
                </div>
            </form>
            <dialog id="share_dialog" onclose={saved_url_onclose}>
                <h3> {"Editor State URL"} </h3>
                <p id="generated-url">{saved_url.as_ref().map(Rc::clone)}</p>
                <div class="dialog-buttons">
                    <button type="button" onclick={saved_url_copy} autofocus=true>
                        {"copy"}
                    </button>
                    <button type="button" onclick={saved_url_close}>
                        {"close"}
                    </button>
                </div>
            </dialog>
        </div>
    }
}

const THEME_SOURCE_KEY: &str = "play-rinja-theme";
const STRUCT_SOURCE_KEY: &str = "play-rinja-struct";
const TMPL_SOURCE_KEY: &str = "play-rinja-template";

fn local_storage() -> Option<Storage> {
    window()?.local_storage().unwrap_or_default()
}

fn save_to_local_storage(storage: &Storage, key: &str, data: &str) {
    if let Ok(data) = JSON::stringify(&JsValue::from_str(data)) {
        if let Some(data) = data.as_string() {
            // Doesn't matter whether or not it succeeded.
            let _ = storage.set_item(key, &data);
        }
    }
}

fn share_dialog() -> Option<HtmlDialogElement> {
    Some(
        window()?
            .document()?
            .get_element_by_id("share_dialog")?
            .unchecked_into(),
    )
}

// Read last editor state from local storage.
// Then delete the known editor state.
// Then, if the app did not crash while processing the retrieved state, save it again.
fn get_last_editor_state() -> Option<(Option<Rc<str>>, Option<Rc<str>>, Option<Rc<str>>)> {
    let window = window()?;
    let storage = window.local_storage().ok().flatten()?;

    let mut theme = None;
    let mut rust = None;
    let mut tmpl = None;
    let mut raw_theme = None;
    let mut raw_rust = None;
    let mut raw_tmpl = None;

    for (key, raw_dest, dest) in [
        (THEME_SOURCE_KEY, &mut raw_theme, &mut theme),
        (STRUCT_SOURCE_KEY, &mut raw_rust, &mut rust),
        (TMPL_SOURCE_KEY, &mut raw_tmpl, &mut tmpl),
    ] {
        let Some(raw) = storage.get_item(key).ok().flatten() else {
            continue;
        };
        let _ = storage.remove_item(key);
        let Some(parsed) = JSON::parse(&raw).ok().and_then(|s| s.as_string()) else {
            continue;
        };
        *raw_dest = Some(raw);
        *dest = Some(Rc::from(parsed));
    }
    if theme.is_none() && rust.is_none() && theme.is_none() {
        return None;
    }

    let callback = Closure::once(move || {
        if crate::PANICKED.load(std::sync::atomic::Ordering::Acquire) {
            return;
        }

        for (key, value) in [
            (THEME_SOURCE_KEY, raw_theme.take()),
            (STRUCT_SOURCE_KEY, raw_rust.take()),
            (TMPL_SOURCE_KEY, raw_tmpl.take()),
        ] {
            if let Some(value) = value {
                let _ = storage.set_item(key, &value);
            }
        }
    });
    let _ = window.set_timeout_with_callback(callback.into_js_value().unchecked_ref());

    Some((theme, rust, tmpl))
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
    fn gen_saved_url(callback: &Function);
    fn read_saved_url(callback: &Function);
    fn save_clipboard(text: &str);
    fn toggle_element(event: MouseEvent, elementId: &str);
    fn handle_blur(event: FocusEvent, elementId: &str);
    fn reset_code(event: MouseEvent, text: &str);
}
