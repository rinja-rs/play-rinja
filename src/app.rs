use std::rc::Rc;
use std::time::Duration;

use prettyplease::unparse;
use proc_macro2::TokenStream;
use rinja_derive_standalone::derive_template;
use syn::{parse2, parse_quote};
use web_sys::wasm_bindgen::prelude::Closure;
use web_sys::wasm_bindgen::JsCast;
use web_sys::{window, HtmlSelectElement};
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

#[function_component]
pub fn App() -> Html {
    let state = use_state(|| {
        let theme = ASSETS
            .1
            .iter()
            .position(|&(theme, _)| theme == "Monokai Extended Bright")
            .unwrap_or_default();
        let (code, duration) = convert_source(STRUCT_SOURCE, TMPL_SOURCE);
        Props {
            theme,
            rust: Rc::from(STRUCT_SOURCE),
            tmpl: Rc::from(TMPL_SOURCE),
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

    let oninput = |edit: fn(&mut Props, String)| {
        let state = state.clone();
        move |data: String| {
            let mut new_state = Props::clone(&*state);
            edit(&mut new_state, data);
            replace_timeout(&mut new_state, state.clone());
            state.set(new_state);
        }
    };
    let oninput_rust = oninput(|new_state, data| new_state.rust = Rc::from(data));
    let oninput_tmpl = oninput(|new_state, data| new_state.tmpl = Rc::from(data));

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
            <div class="top">
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
            <div>
                <label>
                    <strong> {"Theme: "} </strong>
                    <select onchange={onchange_theme}> {themes} </select>
                </label>
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
    let mut code: TokenStream = parse_quote! { #[template(ext = "html", source = #tmpl)] };
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

const TMPL_SOURCE: &str = r##"<div class="example-wrap"> {# #}
    <div data-nosnippet><pre class="src-line-numbers">
        {% for line in lines.clone() %}
            {% if embedded %}
                <span>{{line|safe}}</span>
            {%~ else %}
                <a href="#{{line|safe}}" id="{{line|safe}}">{{line|safe}}</a>
            {%~ endif %}
        {% endfor %}
    </pre></div> {# #}
    <pre class="rust"> {# #}
        <code>
            {% if needs_expansion %}
                <button class="expand">&varr;</button>
            {% endif %}
            {{code_html|safe}}
        </code> {# #}
    </pre> {# #}
</div>"##;

const STRUCT_SOURCE: &str = r##"struct Source<Code: std::fmt::Display> {
    embedded: bool,
    needs_expansion: bool,
    lines: RangeInclusive<usize>,
    code_html: Code,
}"##;
