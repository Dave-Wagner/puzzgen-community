use puzzgen::accounts::AccountStore;
use puzzgen::api::{AppState, Config, router};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "puzzgen=info,tower_http=info".into()),
        )
        .init();
    let config = Config::from_env()?;
    let command = std::env::args().nth(1).unwrap_or_else(|| "serve".into());
    if !matches!(
        command.as_str(),
        "serve" | "migrate" | "create-admin" | "cleanup" | "reset-password"
    ) {
        return Err("Usage: puzzgen [serve|migrate|create-admin|cleanup|reset-password]".into());
    }
    let accounts = if let Ok(database_url) = std::env::var("DATABASE_URL") {
        let origin = std::env::var("PUZZGEN_PUBLIC_ORIGIN")
            .unwrap_or_else(|_| format!("http://{}", config.listen));
        let signup = match std::env::var("PUZZGEN_REGISTRATION_OPEN").as_deref() {
            Ok("false") => false,
            Ok("true") | Err(_) => true,
            Ok(_) => return Err("PUZZGEN_REGISTRATION_OPEN must be true or false".into()),
        };
        let mut store = AccountStore::connect(&database_url, &origin, signup).await?;
        if let Ok(project) = std::env::var("PUZZGEN_FIREBASE_PROJECT_ID") {
            store = store.with_firebase(puzzgen::firebase::FirebaseVerifier::new(project)?);
        }
        Some(store)
    } else {
        if std::env::var_os("PUZZGEN_FIREBASE_PROJECT_ID").is_some() {
            return Err("Firebase guest access requires DATABASE_URL and account quotas".into());
        }
        None
    };
    if command != "serve" {
        let store = accounts
            .as_ref()
            .ok_or("DATABASE_URL is required for account administration")?;
        if command == "migrate" {
            store.migrate().await?;
            println!("Account migrations applied.");
        } else if command == "cleanup" {
            store.check_schema().await?;
            store.cleanup().await?;
            println!("Expired credentials and old account maintenance records removed.");
        } else {
            store.check_schema().await?;
            use std::io::{self, Write};
            let resetting = command == "reset-password";
            print!("Account email: ");
            io::stdout().flush()?;
            let mut email = String::new();
            io::stdin().read_line(&mut email)?;
            let mut name = String::new();
            if !resetting {
                print!("Display name: ");
                io::stdout().flush()?;
                io::stdin().read_line(&mut name)?;
            }
            let password = rpassword::prompt_password("Passphrase (at least 15 characters): ")?;
            let confirmation = rpassword::prompt_password("Confirm passphrase: ")?;
            if password != confirmation {
                return Err("Passphrases did not match".into());
            }
            if resetting {
                store
                    .reset_password(email.trim(), password)
                    .await
                    .map_err(|_| "Could not reset password; check the account and passphrase")?;
                println!("Password reset; existing sessions and keys revoked.");
            } else {
                store.bootstrap_owner(email.trim(), name.trim(), password).await.map_err(|_| "Could not create owner; check input and ensure no owner already exists")?;
                println!("Protected owner created. Sign in through the account page.");
            }
        }
        return Ok(());
    }
    if let Some(store) = &accounts {
        store
            .check_schema()
            .await
            .map_err(|_| "Account schema unavailable; run puzzgen migrate first")?;
        if !store.owner_exists().await? {
            return Err(
                "No owner exists; run puzzgen create-admin before starting the member service"
                    .into(),
            );
        }
    }
    let listener = tokio::net::TcpListener::bind(config.listen).await?;
    tracing::info!(address = %listener.local_addr()?, workers = config.workers, auth_enabled = accounts.is_some() || !config.api_keys.is_empty(), "puzzgen listening");
    axum::serve(
        listener,
        router(AppState::with_accounts(config, accounts))
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown())
    .await?;
    Ok(())
}

async fn shutdown() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("shutting down");
}
