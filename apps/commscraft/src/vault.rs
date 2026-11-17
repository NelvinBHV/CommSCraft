//! The password in the Linux keyring, only when the user asks for it ("remember the password"):
//! the Secret Service over D-Bus (GNOME Keyring, KWallet's service, KeePassXC …) or, inside a
//! sandbox, the keyring portal – through `oo7` on zbus 6.
//!
//! Security by design: nothing is stored without the user's choice and only after a successful
//! sign-in; one item per sign-in address (attributes `application=commscraft`, `account=<address>`);
//! the secret is held in zeroized memory; a keyring that is missing or locked only means the
//! password is asked for. The password never goes to a file of our own.

use zeroize::Zeroizing;

const APPLICATION: &str = "commscraft";

fn attributes(account: &str) -> [(&'static str, &str); 2] {
    [("application", APPLICATION), ("account", account)]
}

fn block<T>(f: impl std::future::Future<Output = T>) -> T {
    futures_lite::future::block_on(f)
}

/// Stores the password of a sign-in address (replacing an earlier one).
pub fn store(account: &str, password: &str) -> Result<(), String> {
    block(async {
        let k = oo7::Keyring::new().await.map_err(|e| e.to_string())?;
        k.unlock().await.map_err(|e| e.to_string())?;
        k.create_item(
            &format!("CommSCraft: {account}"),
            &attributes(account),
            oo7::Secret::text(password),
            true,
        )
        .await
        .map_err(|e| e.to_string())
    })
}

/// The stored password of a sign-in address, if there is one.
pub fn load(account: &str) -> Option<Zeroizing<String>> {
    block(async {
        let k = oo7::Keyring::new().await.ok()?;
        let items = k.search_items(&attributes(account)).await.ok()?;
        let item = items.first()?;
        let _ = item.unlock().await;
        let secret = item.secret().await.ok()?;
        let text = std::str::from_utf8(secret.as_bytes()).ok()?;
        Some(Zeroizing::new(text.to_string()))
    })
}

/// Whether a password is stored for a sign-in address (no secret is read).
pub fn has(account: &str) -> bool {
    block(async {
        match oo7::Keyring::new().await {
            Ok(k) => k
                .search_items(&attributes(account))
                .await
                .is_ok_and(|i| !i.is_empty()),
            Err(_) => false,
        }
    })
}

/// Removes the stored password of a sign-in address.
pub fn forget(account: &str) -> Result<(), String> {
    block(async {
        let k = oo7::Keyring::new().await.map_err(|e| e.to_string())?;
        k.delete(&attributes(account))
            .await
            .map_err(|e| e.to_string())
    })
}
