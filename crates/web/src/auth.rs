//! Auth context — the Leptos counterpart of `AuthContext.tsx`.
//!
//! `AuthCtx` is `Copy` (every field is a `RwSignal`, which is `Copy` in Leptos
//! 0.8), so components take it by value out of context without cloning games.

use ft_schema::api::UserProfile;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;

#[derive(Clone, Copy)]
pub struct AuthCtx {
    pub user: RwSignal<Option<UserProfile>>,
    /// True until the initial session probe settles, so guards don't bounce a
    /// logged-in user to /login on first paint.
    pub loading: RwSignal<bool>,
}

impl AuthCtx {
    pub fn new() -> Self {
        Self {
            user: RwSignal::new(None),
            loading: RwSignal::new(true),
        }
    }

    /// Re-fetch `/api/users/me`. On failure the session is dropped, mirroring the
    /// React `refreshUser` catch branch.
    pub async fn refresh(self, no_cache: bool) {
        match api::get_me(no_cache).await {
            Ok(u) => self.user.set(Some(u)),
            Err(_) => {
                api::set_session(None);
                self.user.set(None);
            }
        }
    }

    /// Probe an existing localStorage session once at startup.
    pub fn bootstrap(self) {
        if api::get_session().is_none() {
            self.loading.set(false);
            return;
        }
        spawn_local(async move {
            self.refresh(false).await;
            self.loading.set(false);
        });
    }

    pub fn logout(self) {
        spawn_local(async move {
            api::logout().await;
            self.user.set(None);
        });
    }

    pub fn is_authed(self) -> bool {
        self.user.get().is_some()
    }
}

impl Default for AuthCtx {
    fn default() -> Self {
        Self::new()
    }
}

/// Pull `AuthCtx` out of context. Panics only if `App` failed to provide it,
/// which is a wiring bug rather than a runtime condition.
pub fn use_auth() -> AuthCtx {
    use_context::<AuthCtx>().expect("AuthCtx must be provided by <App/>")
}
