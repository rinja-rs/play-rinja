mod app;
mod editor;

use crate::app::App;

fn main() {
    yew::Renderer::<App>::new().render();
}
