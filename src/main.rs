mod app;
mod editor;

use std::panic::Location;

use once_cell::sync::Lazy;
use syntect::highlighting::Theme;
use syntect::parsing::SyntaxSet;
use syntect_assets::assets::HighlightingAssets;
use web_sys::js_sys::Error;
use web_sys::wasm_bindgen::throw_val;

use crate::app::App;

fn main() {
    yew::Renderer::<App>::new().render();
}

trait ThrowAt<T> {
    fn unwrap_at(self) -> T;
}

impl<T> ThrowAt<T> for Option<T> {
    #[track_caller]
    #[inline]
    fn unwrap_at(self) -> T {
        #[cold]
        #[inline(never)]
        fn fail(location: &Location<'_>) -> ! {
            throw_val(Error::new(&format!("unwrap failed @ {location}")).into())
        }

        match self {
            Some(value) => value,
            None => fail(Location::caller()),
        }
    }
}

impl<T, E: std::fmt::Display> ThrowAt<T> for Result<T, E> {
    #[track_caller]
    #[inline]
    fn unwrap_at(self) -> T {
        #[cold]
        #[inline(never)]
        fn fail<E: std::fmt::Display>(location: &Location<'_>, err: E) -> ! {
            throw_val(Error::new(&format!("unwrap failed @ {location}: {err}")).into())
        }

        match self {
            Ok(value) => value,
            Err(err) => fail(Location::caller(), err),
        }
    }
}

static ASSETS: Lazy<(&SyntaxSet, &[(&str, &Theme)])> = Lazy::new(|| {
    let assets = Box::leak(Box::new(HighlightingAssets::from_binary()));

    let mut themes = assets
        .themes()
        .filter_map(
            |theme| match ["ansi", "base16", "base16-256"].contains(&theme) {
                true => None,
                false => Some((theme, assets.get_theme(theme))),
            },
        )
        .collect::<Box<[(_, _)]>>();
    themes.sort_unstable_by(|&(l, _), &(r, _)| l.to_lowercase().cmp(&r.to_lowercase()));
    let themes = Box::leak(themes);

    let syntax_set = assets.get_syntax_set().unwrap_at();
    (syntax_set, themes)
});
