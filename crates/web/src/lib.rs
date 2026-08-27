//! ft-web — Leptos CSR frontend. Mirrors the React app in `frontend/` route for
//! route, sharing wire types with the Worker via `ft-schema::api`.

pub mod api;
pub mod auth;
pub mod components;
pub mod pages;

use leptos::prelude::*;
use leptos_router::components::{Redirect, Route, Router, Routes};
use leptos_router::path;

use crate::auth::{use_auth, AuthCtx};
use crate::components::Layout;
use crate::pages::{DivinationPage, HomePage, LoginPage, ProfilePage, StoryPage};

#[component]
pub fn App() -> impl IntoView {
    let auth = AuthCtx::new();
    provide_context(auth);
    auth.bootstrap();

    view! {
        <Router>
            <Layout>
                <Routes fallback=|| view! { <div class="center-note">"找不到頁面"</div> }>
                    <Route path=path!("/") view=HomePage/>
                    <Route path=path!("/login") view=LoginPage/>
                    <Route path=path!("/profile") view=|| view! { <Protected><ProfilePage/></Protected> }/>
                    <Route path=path!("/divination/:type") view=|| view! { <Protected><DivinationPage/></Protected> }/>
                    <Route path=path!("/story") view=|| view! { <Protected><StoryPage/></Protected> }/>
                </Routes>
            </Layout>
        </Router>
    }
}

/// Route guard — the counterpart of `ProtectedRoute.tsx`. Waits for the initial
/// session probe before deciding, so a logged-in reload doesn't flash /login.
#[component]
fn Protected(children: ChildrenFn) -> impl IntoView {
    let auth = use_auth();
    move || {
        if auth.loading.get() {
            view! { <div class="center-note">"Loading..."</div> }.into_any()
        } else if auth.is_authed() {
            children().into_any()
        } else {
            view! { <Redirect path="/login"/> }.into_any()
        }
    }
}

/// WASM entry. Leptos CSR: mount the app to the body when the module loads.
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
