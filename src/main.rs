use std::rc::Rc;
use std::time::Duration;

use prettyplease::unparse;
use proc_macro2::TokenStream;
use syn::{parse2, parse_quote};
use web_sys::js_sys::global;
use web_sys::wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{HtmlTextAreaElement, WorkerGlobalScope};
use yew::{
    function_component, html, use_state, Callback, Html, InputEvent, SubmitEvent, UseStateHandle,
};

fn main() {
    yew::Renderer::<App>::new().render();
}

#[function_component]
fn App() -> Html {
    let struct_source = use_state(|| Rc::<str>::from(STRUCT_SOURCE));
    let tmpl_source = use_state(|| Rc::<str>::from(TMPL_SOURCE));

    let performance = global().unchecked_into::<WorkerGlobalScope>().performance();
    let start = performance.as_ref().map_or(0.0, |p| p.now());
    let generated_source = {
        let source: &str = &tmpl_source;
        let mut source: TokenStream = parse_quote! { #[template(ext = "html", source = #source)] };
        source.extend(struct_source.parse::<TokenStream>());
        let source = rinja_derive_standalone::derive_template(source);
        let source = parse2(source).unwrap();
        unparse(&source)
    };
    let duration = performance.as_ref().map_or(None, |p| {
        let ms = (p.now() - start).max(0.0);
        Some(format!(" ({:?})", Duration::from_secs_f64(ms / 1000.0)))
    });

    let onchange = |state: &UseStateHandle<Rc<str>>| {
        let state = state.clone();
        Callback::from(move |ev: InputEvent| {
            state.set(
                ev.target()
                    .unwrap_throw()
                    .unchecked_into::<HtmlTextAreaElement>()
                    .value()
                    .into(),
            );
        })
    };

    let onsubmit = Callback::from(|ev: SubmitEvent| {
        ev.prevent_default();
    });

    html! {
        <form method="GET" action="javascript:;" onsubmit={onsubmit}>
            <p>
                <label>
                    <strong> {"Your struct:"} </strong><br />
                    <textarea
                        rows="10"
                        spellcheck="false"
                        value={Rc::clone(&struct_source)}
                        oninput={onchange(&struct_source)}
                    />
                </label>
            </p>
            <p>
                <label>
                    <strong> {"Your template:"} </strong><br />
                    <textarea
                        rows="10"
                        spellcheck="false"
                        value={Rc::clone(&tmpl_source)}
                        oninput={onchange(&tmpl_source)}
                    />
                </label>
            </p>
            <p>
                <label>
                    <strong> {"Generated code:"} {duration} </strong><br />
                    <textarea
                        rows="20"
                        spellcheck="false"
                        readonly=true
                        value={generated_source}
                    />
                </label>
            </p>
        </form>
    }
}

const TMPL_SOURCE: &str = r##"<div class="example-wrap"> {# #}
    {# https://developers.google.com/search/docs/crawling-indexing/robots-meta-tag#data-nosnippet-attr
       Do not show "1 2 3 4 5 ..." in web search results. #}
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
