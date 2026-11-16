//! Seed only a disposable local *_test database for browser QA.
//! These public test credentials must never be used for a real account.
use puzzgen::accounts::AccountStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database = std::env::var("DATABASE_URL")?;
    let url = url::Url::parse(&database)?;
    if !url.path().ends_with("_test") {
        return Err("Demo seeding requires a disposable database ending in _test".into());
    }
    let store = AccountStore::connect(&database, "http://127.0.0.1:8080", true).await?;
    store.migrate().await?;
    store
        .bootstrap_owner(
            "owner@example.test",
            "Demo Owner",
            "a unique integration test passphrase".into(),
        )
        .await
        .map_err(|_| "Could not seed a demo owner; use a fresh test database")?;
    println!(
        "Created the local test owner owner@example.test. Its test passphrase is in examples/seed_accounts_demo.rs."
    );
    Ok(())
}
