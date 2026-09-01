//! ft-web — Leptos CSR frontend. Mirrors the React app in `frontend/` route for
//! route, sharing wire types with the Worker via `ft-schema::api`.

pub mod api;
pub mod auth;
pub mod components;
pub mod generation;
pub mod pages;

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::{Redirect, Route, Router, Routes};
use leptos_router::hooks::use_navigate;
use leptos_router::path;

use crate::auth::{use_auth, AuthCtx};
use crate::components::Layout;
use crate::pages::{
    AdminPage, DivinationPage, HomePage, LoginPage, PersonalityPage, ProfilePage, StoryPage,
};

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
                    // Same page as /login: the admin invite URL says /register
                    // (built backend-side), which reads better in an invite.
                    <Route path=path!("/register") view=LoginPage/>
                    <Route path=path!("/auth/verify") view=VerifyPage/>
                    <Route path=path!("/profile") view=|| view! { <Protected><ProfilePage/></Protected> }/>
                    <Route path=path!("/personality") view=|| view! { <Protected><PersonalityPage/></Protected> }/>
                    <Route path=path!("/divination/:type") view=|| view! { <Protected><DivinationPage/></Protected> }/>
                    <Route path=path!("/story") view=|| view! { <Protected><StoryPage/></Protected> }/>
                    <Route path=path!("/admin") view=|| view! { <Protected><AdminPage/></Protected> }/>
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

/// `/auth/verify` — the landing page of the magic-link email. Exchanges the
/// `?token=` query param for a session via `api::verify_email`, then continues
/// to /profile. Public on purpose: this is how an anonymous visitor becomes
/// authenticated. Lives here rather than in `pages/` because `pages/mod.rs`
/// re-exports only the pages lib.rs already names, and this file owns routing.
#[component]
fn VerifyPage() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();
    let failed = RwSignal::new(false);
    let reason = RwSignal::new(String::new());

    Effect::new(move |_| {
        let navigate = navigate.clone();
        spawn_local(async move {
            match verify_token_from_url() {
                None => {
                    reason.set("連結缺少驗證碼（token）。".to_string());
                    failed.set(true);
                }
                Some(token) => match api::verify_email(&token).await {
                    // `verify_email` already stored the session; refresh the
                    // user signal so `Protected` sees an authenticated visitor.
                    Ok(_) => {
                        auth.refresh(false).await;
                        navigate("/profile", Default::default());
                    }
                    Err(e) => {
                        reason.set(e.to_string());
                        failed.set(true);
                    }
                },
            }
        });
    });

    view! {
        <div class="auth-page">
            <div class="card">
                <Show
                    when=move || !failed.get()
                    fallback=move || view! {
                        <h2 style="margin-bottom:1rem;text-align:center">"登入連結無效或已過期"</h2>
                        <p class="error">{move || reason.get()}</p>
                        <p style="font-size:0.8rem;color:#6b7280;line-height:1.8;text-align:center">
                            "連結只能使用一次，且會在數分鐘後失效。請回到登入頁重新申請一封登入信。"
                        </p>
                        <div style="margin-top:1.5rem;text-align:center">
                            <a href="/login" class="btn-link">"返回登入頁"</a>
                        </div>
                    }
                >
                    <div class="center-note">"正在驗證登入連結..."</div>
                </Show>
            </div>
        </div>
    }
}

/// Read `token` out of `window.location.search`. Leptos CSR: the query string
/// only exists in the browser. The backend mints hex tokens, so a plain split
/// needs no percent-decoding.
fn verify_token_from_url() -> Option<String> {
    query_param("token")
}

/// One query parameter from `window.location.search`, generic over names so
/// the invite prefill (`?invite=`) shares the same parsing. Read through
/// `js_sys::Reflect` because `web_sys::Location` would require a `Location`
/// feature this crate's `web-sys` dependency does not enable.
pub(crate) fn query_param(name: &str) -> Option<String> {
    let win = web_sys::window()?;
    let loc = js_sys::Reflect::get(win.as_ref(), &"location".into()).ok()?;
    let search = js_sys::Reflect::get(&loc, &"search".into())
        .ok()?
        .as_string()?;
    let query = search.strip_prefix('?')?;
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == name && !v.is_empty()).then(|| v.to_string())
    })
}

/// WASM entry. Leptos CSR: mount the app to the body when the module loads.
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
