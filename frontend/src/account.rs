use gloo_net::http::Request;
use leptos::prelude::*;
use serde_json::{Value, json};
use wasm_bindgen_futures::spawn_local;

async fn call(path: &str, body: Option<Value>, csrf: &str) -> Result<Value, String> {
    let url = format!("/api/v1/account/{path}");
    let request = if body.is_some() {
        Request::post(&url)
    } else {
        Request::get(&url)
    };
    let request = if !csrf.is_empty() {
        request.header("X-CSRF-Token", csrf)
    } else {
        request
    };
    let response = if let Some(body) = body {
        request.json(&body).map_err(|e| e.to_string())?.send().await
    } else {
        request.send().await
    }
    .map_err(|_| "Could not reach account services".to_string())?;
    let value: Value = response
        .json()
        .await
        .map_err(|_| "Unreadable account response".to_string())?;
    if !response.ok() {
        return Err(value
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("Account request failed")
            .into());
    }
    Ok(value)
}

#[derive(Clone, Copy)]
struct AccountUi {
    csrf: RwSignal<String>,
    user: RwSignal<Option<Value>>,
    usage: RwSignal<Value>,
    keys: RwSignal<Vec<Value>>,
    members: RwSignal<Vec<Value>>,
    busy: RwSignal<bool>,
    error: RwSignal<String>,
    message: RwSignal<String>,
    secret: RwSignal<String>,
}
impl AccountUi {
    fn clear(self) {
        self.csrf.set(String::new());
        self.user.set(None);
        self.keys.set(Vec::new());
        self.members.set(Vec::new());
        self.secret.set(String::new());
    }
    async fn refresh(self) {
        match call("session", None, "").await {
            Ok(session) => {
                self.csrf
                    .set(session["csrf_token"].as_str().unwrap_or("").into());
                self.usage.set(session["usage"].clone());
                self.user.set(Some(session["user"].clone()));
                if let Ok(value) = call("keys", None, "").await {
                    self.keys
                        .set(value["keys"].as_array().cloned().unwrap_or_default());
                }
                if matches!(session["user"]["role"].as_str(), Some("owner" | "admin")) {
                    match call("admin/members", None, "").await {
                        Ok(value) => self
                            .members
                            .set(value["members"].as_array().cloned().unwrap_or_default()),
                        Err(error) => self.error.set(error),
                    }
                } else {
                    self.members.set(Vec::new());
                }
            }
            Err(_) => self.clear(),
        }
    }
    fn submit(self, path: String, body: Value, success: &'static str) {
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        self.error.set(String::new());
        self.message.set(String::new());
        spawn_local(async move {
            match call(&path, Some(body), &self.csrf.get_untracked()).await {
                Ok(value) => {
                    if let Some(secret) = value["secret"].as_str() {
                        self.secret.set(secret.into());
                    }
                    if path == "logout" || path == "delete" || path == "sessions/revoke" {
                        self.clear();
                    } else {
                        self.refresh().await;
                    }
                    self.message.set(success.into());
                }
                Err(error) => self.error.set(error),
            }
            self.busy.set(false);
        });
    }
}

#[component]
pub fn AccountPanel(csrf: RwSignal<String>) -> impl IntoView {
    let enabled = RwSignal::new(None::<bool>);
    let registration = RwSignal::new(false);
    let registering = RwSignal::new(false);
    let email = RwSignal::new(String::new());
    let name = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let profile_name = RwSignal::new(String::new());
    let current_password = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());
    let key_name = RwSignal::new(String::new());
    let delete_password = RwSignal::new(String::new());
    let delete_confirmation = RwSignal::new(String::new());
    let ui = AccountUi {
        csrf,
        user: RwSignal::new(None),
        usage: RwSignal::new(Value::Null),
        keys: RwSignal::new(Vec::new()),
        members: RwSignal::new(Vec::new()),
        busy: RwSignal::new(false),
        error: RwSignal::new(String::new()),
        message: RwSignal::new(String::new()),
        secret: RwSignal::new(String::new()),
    };
    spawn_local(async move {
        match call("config", None, "").await {
            Ok(config) => {
                enabled.set(Some(config["enabled"].as_bool().unwrap_or(false)));
                registration.set(config["registration_open"].as_bool().unwrap_or(false));
                if config["enabled"] == true {
                    ui.refresh().await;
                }
            }
            Err(error) => {
                enabled.set(Some(false));
                ui.error.set(error);
            }
        }
    });
    view! {
        <section class="account-hub" id="account" aria-labelledby="account-title">
            <div class="eyebrow"><span class="tiny-square"></span>"YOUR PUZZGEN ACCOUNT"</div><h2 id="account-title">"A home for your projects."</h2>
            <p class="learning-intro">"Manage your profile, keep your API keys organized, and see the resources available to your account."</p>
            <Show when=move || !ui.error.get().is_empty()><p class="error-banner" role="alert">{move ||ui.error.get()}</p></Show>
            <Show when=move || !ui.message.get().is_empty()><p class="account-success" role="status">{move ||ui.message.get()}</p></Show>
            {move || match enabled.get(){
                None=>view!{<p>"Connecting to accounts…"</p>}.into_any(),
                Some(false)=>view!{<p class="learning-note">"Member accounts have not been configured on this instance. The offline learning resources are available without an account."</p>}.into_any(),
                Some(true)=>view!{
                    <Show when=move ||ui.user.get().is_none() fallback=move ||view!{
                        <div class="account-toolbar"><div><strong>{move ||ui.user.get().map(|u|u["display_name"].as_str().unwrap_or("").to_owned()).unwrap_or_default()}</strong><span>{move ||ui.user.get().map(|u|format!("{} · {}",u["email"].as_str().unwrap_or(""),u["role"].as_str().unwrap_or(""))).unwrap_or_default()}</span></div><button class="recipe-tab" disabled=move ||ui.busy.get() on:click=move |_|ui.submit("logout".into(),json!({}),"Signed out")>"Sign out"</button></div>
                        <div class="account-grid">
                            <article class="account-card"><h3>"Your allocation"</h3><p class="learning-note">"All keys and the playground share these limits. Daily operations reset at midnight UTC. Checking an answer costs one operation; a batch costs one per puzzle."</p>
                                <dl class="quota-list">{[ ("requests_per_minute","Requests / minute"),("operations_per_day","Operations / day"),("max_keys","Active API keys"),("max_batch","Puzzles / batch"),("max_concurrent","Concurrent operations") ].into_iter().map(|(field,label)|view!{<div><dt>{label}</dt><dd>{move ||ui.user.get().map(|u|if u["limits"][field].is_null(){"Unlimited".into()}else{u["limits"][field].to_string()}).unwrap_or_default()}</dd></div>}).collect_view()}</dl>
                                <p class="usage-line">{move ||format!("Used today: {} operations · {} requests this minute",ui.usage.get()["operations_today"].as_i64().unwrap_or(0),ui.usage.get()["requests_this_minute"].as_i64().unwrap_or(0))}</p><button class="text-button" disabled=move ||ui.busy.get() on:click=move |_|{ui.busy.set(true);spawn_local(async move{ui.refresh().await;ui.busy.set(false);});}>"Refresh usage ↻"</button>
                            </article>
                            <article class="account-card"><h3>"Profile"</h3><p class="learning-note">"Your email is your sign-in identifier. Display names appear only in account administration."</p>
                                <form class="account-form" on:submit=move |ev|{ev.prevent_default();ui.submit("profile".into(),json!({"display_name":profile_name.get()}),"Profile updated");}>
                                    <label for="profile-name">"Display name"</label><input id="profile-name" required maxlength="80" placeholder=move ||ui.user.get().map(|u|u["display_name"].as_str().unwrap_or("").to_owned()).unwrap_or_default() prop:value=move ||profile_name.get() on:input=move |ev|profile_name.set(event_target_value(&ev))/><button class="generate-button" disabled=move ||ui.busy.get()>"Save profile"</button>
                                </form>
                            </article>
                        </div>
                        <article class="account-card account-keys"><h3>"API keys"</h3><p class="learning-note">"Regular accounts can have three active keys. A secret is shown once; give each project its own key, and revoke keys you no longer use. Never commit a key to source control."</p>
                            <form class="account-form inline-account-form" on:submit=move |ev|{ev.prevent_default();ui.secret.set(String::new());ui.submit("keys".into(),json!({"name":key_name.get()}),"Key created. Save its secret now.");}><div><label for="key-name">"Key name"</label><input id="key-name" required maxlength="80" placeholder="My learning project" prop:value=move ||key_name.get() on:input=move |ev|key_name.set(event_target_value(&ev))/></div><button class="generate-button" disabled=move ||ui.busy.get()>"Create API key"</button></form>
                            <Show when=move ||!ui.secret.get().is_empty()><div class="key-secret" role="status"><strong>"Save this secret now. It cannot be retrieved later."</strong><code>{move ||ui.secret.get()}</code><button class="recipe-tab" on:click=move |_|ui.secret.set(String::new())>"I saved it — hide secret"</button></div></Show>
                            <div class="key-list">{move ||ui.keys.get().into_iter().map(|key|{let id=key["id"].as_str().unwrap_or("").to_owned();view!{<div class="key-row"><div><strong>{key["name"].as_str().unwrap_or("").to_owned()}</strong><code>{format!("{}…",key["prefix"].as_str().unwrap_or(""))}</code><small>{format!("Created {}",key["created_at"].as_str().unwrap_or(""))}</small></div><button class="recipe-tab" disabled=move ||ui.busy.get() on:click=move |_|ui.submit(format!("keys/{id}/revoke"),json!({}),"API key revoked")>"Revoke"</button></div>}}).collect_view()}</div>
                            <pre class="learning-code"><code>{"fetch(`${apiOrigin}/api/v1/puzzles/maze?seed=42`, {\n  headers: { Authorization: `Bearer ${yourKey}` }\n});"}</code></pre>
                        </article>
                        <details class="account-card"><summary>"Password & account security"</summary>
                            <form class="account-form" on:submit=move |ev|{ev.prevent_default();let body=json!({"current_password":current_password.get(),"new_password":new_password.get()});current_password.set(String::new());new_password.set(String::new());ui.submit("password".into(),body,"Password changed; previous sessions signed out");}>
                                <label for="current-password">"Current password"</label><input id="current-password" type="password" autocomplete="current-password" required prop:value=move ||current_password.get() on:input=move |ev|current_password.set(event_target_value(&ev))/><label for="new-password">"New passphrase — at least 15 characters"</label><input id="new-password" type="password" autocomplete="new-password" required minlength="15" maxlength="1024" prop:value=move ||new_password.get() on:input=move |ev|new_password.set(event_target_value(&ev))/><button class="generate-button" disabled=move ||ui.busy.get()>"Change password"</button>
                            </form><p class="learning-note">"Changing your password signs out existing sessions. API keys are separate credentials; revoke them if they may have been exposed."</p><button class="recipe-tab" disabled=move ||ui.busy.get() on:click=move |_|ui.submit("sessions/revoke".into(),json!({}),"Signed out all sessions, including this one")>"Sign out all sessions"</button>
                        </details>
                        <Show when=move ||ui.user.get().is_some_and(|u|matches!(u["role"].as_str(),Some("admin"|"owner")))>
                            <section class="admin-panel" aria-labelledby="admin-title"><div class="eyebrow">"ADMINISTRATION"</div><h3 id="admin-title">"Members & resource limits"</h3><p class="learning-note">"Blank limits mean unlimited; zero disables that resource. These are account quotas. The server still bounds batch size and available CPU workers. Only the owner can assign roles, and the owner cannot be disabled or demoted."</p>
                                {move ||ui.members.get().into_iter().map(|member|view!{<MemberEditor member=member ui=ui/>}).collect_view()}
                            </section>
                        </Show>
                        <Show when=move ||ui.user.get().is_some_and(|u|u["role"]!="owner")>
                            <details class="account-card danger-zone"><summary>"Delete my account"</summary><p class="learning-note">"This permanently removes your profile, sessions, API keys, and usage records. Enter your password and type DELETE to confirm."</p><form class="account-form" on:submit=move |ev|{ev.prevent_default();let body=json!({"password":delete_password.get(),"confirmation":delete_confirmation.get()});delete_password.set(String::new());ui.submit("delete".into(),body,"Account deleted");}><label for="delete-password">"Password"</label><input id="delete-password" type="password" autocomplete="current-password" required prop:value=move ||delete_password.get() on:input=move |ev|delete_password.set(event_target_value(&ev))/><label for="delete-confirmation">"Type DELETE"</label><input id="delete-confirmation" required pattern="DELETE" prop:value=move ||delete_confirmation.get() on:input=move |ev|delete_confirmation.set(event_target_value(&ev))/><button class="recipe-tab" disabled=move ||ui.busy.get()>"Permanently delete my account"</button></form></details>
                        </Show>
                    }>
                        <div class="account-card sign-in-card"><h3>{move ||if registering.get(){"Create your free account"}else{"Welcome back"}}</h3><p class="learning-note">"Free accounts start with three keys, 10 requests per minute and 100 puzzle operations per day. Choose a unique passphrase."</p>
                            <form class="account-form" on:submit=move |ev|{ev.prevent_default();let register=registering.get();let body=if register{json!({"email":email.get(),"display_name":name.get(),"password":password.get()})}else{json!({"email":email.get(),"password":password.get()})};password.set(String::new());ui.submit(if register{"register"}else{"login"}.into(),body,"Signed in. Your playground and API keys are ready.");}>
                                <label for="account-email">"Email"</label><input id="account-email" type="email" autocomplete="username" required maxlength="254" prop:value=move ||email.get() on:input=move |ev|email.set(event_target_value(&ev))/>
                                <Show when=move ||registering.get()><label for="account-name">"Display name"</label><input id="account-name" required maxlength="80" autocomplete="nickname" prop:value=move ||name.get() on:input=move |ev|name.set(event_target_value(&ev))/></Show>
                                <label for="account-password">{move ||if registering.get(){"Passphrase — at least 15 characters"}else{"Password"}}</label><input id="account-password" type="password" autocomplete=move ||if registering.get(){"new-password"}else{"current-password"} minlength=move ||if registering.get(){15}else{1} maxlength="1024" required prop:value=move ||password.get() on:input=move |ev|password.set(event_target_value(&ev))/>
                                <button class="generate-button" disabled=move ||ui.busy.get()>{move ||if ui.busy.get(){"Please wait…"}else if registering.get(){"Create account"}else{"Sign in"}}</button>
                            </form><Show when=move ||registration.get()><button class="text-button" disabled=move ||ui.busy.get() on:click=move |_|{registering.update(|v|*v = !*v);ui.error.set(String::new());}>{move ||if registering.get(){"Already have an account? Sign in"}else{"New here? Create a free account"}}</button></Show>
                        </div>
                    </Show>
                }.into_any()
            }}
        </section>
    }
}

const LIMITS: [(&str, &str); 5] = [
    ("requests_per_minute", "Requests / minute"),
    ("operations_per_day", "Operations / day"),
    ("max_keys", "Active keys"),
    ("max_batch", "Batch size"),
    ("max_concurrent", "Concurrency"),
];
#[component]
fn MemberEditor(member: Value, ui: AccountUi) -> impl IntoView {
    let id = member["id"].as_str().unwrap_or("").to_owned();
    let target_role = member["role"].as_str().unwrap_or("member").to_owned();
    let owner = target_role == "owner";
    let actor = ui.user.get_untracked().unwrap_or(Value::Null);
    let actor_owner = actor["role"] == "owner";
    let editable = actor_owner
        || (!matches!(target_role.as_str(), "admin" | "owner") || actor["id"] == member["id"]);
    let role = RwSignal::new(target_role);
    let disabled = RwSignal::new(member["disabled"].as_bool().unwrap_or(false));
    let limits = LIMITS.map(|(field, _)| {
        RwSignal::new(
            member["limits"][field]
                .as_i64()
                .map(|n| n.to_string())
                .unwrap_or_default(),
        )
    });
    let email = member["email"].as_str().unwrap_or("").to_owned();
    let title = member["display_name"].as_str().unwrap_or("").to_owned();
    let revoke_id = id.clone();
    view! {<details class="member-editor"><summary>{format!("{title} · {email}")}</summary><form class="account-form" on:submit=move |ev|{
        ev.prevent_default();let mut body=json!({"role":role.get(),"disabled":disabled.get()});
        for (index,(field,_)) in LIMITS.iter().enumerate(){let raw=limits[index].get();let value=if raw.trim().is_empty(){Value::Null}else{match raw.parse::<i64>(){Ok(n) if (0..=i32::MAX as i64).contains(&n)=>json!(n),_=>{ui.error.set("Use nonnegative whole-number limits, or leave blank for unlimited".into());return;}}};body[*field]=value;}
        ui.submit(format!("admin/members/{id}"),body,"Member allocation updated");
    }>
        <div class="member-fields"><div><label>"Role"<select disabled=move ||ui.busy.get() || !actor_owner || owner prop:value=move ||role.get() on:change=move |ev|role.set(event_target_value(&ev))><option value="member">"Member"</option><option value="trusted">"Trusted member"</option><option value="admin">"Administrator"</option><Show when=move ||owner><option value="owner">"Owner"</option></Show></select></label></div>
            {LIMITS.into_iter().enumerate().map(|(index,(_,label))|view!{<label>{label}<input type="text" inputmode="numeric" pattern="[0-9]*" placeholder="Unlimited" disabled=move ||ui.busy.get() || !editable prop:value=move ||limits[index].get() on:input=move |ev|limits[index].set(event_target_value(&ev))/></label>}).collect_view()}
        </div><label class="checkbox-label"><input type="checkbox" disabled=move ||ui.busy.get() || owner || !editable prop:checked=move ||disabled.get() on:change=move |ev|disabled.set(event_target_checked(&ev))/>{" Disable account (also revokes its keys and sessions)"}</label><button class="generate-button" disabled=move ||ui.busy.get() || !editable>"Save role & limits"</button>
    </form><button class="text-button" disabled=move ||ui.busy.get() || !editable on:click=move |_|ui.submit(format!("admin/members/{revoke_id}/revoke"),json!({}),"Member sessions and keys revoked")>"Revoke all credentials"</button></details>}
}
