use crate::{CommandError, CommandResponse};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;
use vault::Stored;

const ACCOUNT: &str = "app-lock";
const LABEL: &str = "app-lock";

pub(crate) struct Password {
    pub(crate) value: String,
    pub(crate) development: bool,
}

pub(crate) fn read() -> Result<Stored<Password>, vault::Error> {
    #[cfg(debug_assertions)]
    if let Some([value]) = vault::variables(["APP_LOCK_PASSWORD"], "app lock password")? {
        validate(&value)?;
        return Ok(Stored::Ready(Password {
            value,
            development: true,
        }));
    }
    Ok(match vault::read(ACCOUNT)? {
        Stored::Ready(value) => Stored::Ready(Password {
            value,
            development: false,
        }),
        Stored::Missing => Stored::Missing,
    })
}

fn validate(password: &str) -> Result<(), vault::Error> {
    if password.trim().is_empty() {
        return Err(vault::Error::Empty("app lock password"));
    }
    if password.chars().count() < 4 {
        return Err(vault::Error::TooShort("app lock password", 4));
    }
    Ok(())
}

fn configured() -> Result<String, CommandError> {
    match read()? {
        Stored::Ready(password) => Ok(password.value),
        Stored::Missing => Err("App Lock is not configured.".into()),
    }
}

#[derive(Default)]
pub(crate) struct AppLock(AtomicBool);

impl AppLock {
    fn unlock(&self, stored: &str, supplied: &str) -> bool {
        if !passwords_match(stored, supplied) {
            return false;
        }
        self.0.store(false, Ordering::SeqCst);
        true
    }

    pub(crate) fn locked(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[tauri::command]
pub(crate) fn save_app_lock(password: String) -> CommandResponse<String> {
    let result = validate(&password).and_then(|()| vault::save(ACCOUNT, &password));
    CommandResponse::from(result.map(|()| LABEL.to_owned()))
}

#[tauri::command]
pub(crate) fn remove_app_lock() -> CommandResponse<String> {
    CommandResponse::from(vault::delete(ACCOUNT).map(|()| LABEL.to_owned()))
}

#[tauri::command]
pub(crate) fn unlock_app(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppLock>,
    password: String,
) -> CommandResponse<String> {
    let result = configured().and_then(|stored| {
        if !state.unlock(&stored, &password) {
            return Err("Incorrect password.".into());
        }
        crate::island::sync(&app);
        Ok(LABEL.to_owned())
    });
    result.into()
}

#[tauri::command]
pub(crate) fn read_app_lock(state: tauri::State<'_, AppLock>) -> bool {
    state.locked()
}

#[tauri::command]
pub(crate) fn lock_app(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppLock>,
) -> CommandResponse<()> {
    let result = configured().map(|_| {
        state.0.store(true, Ordering::SeqCst);
        app.state::<crate::terminal::Runtime>().suspend();
        app.state::<crate::chat::Runtime>().suspend(&app);
        crate::island::sync(&app);
        if let Some(webview) = app.get_webview_window("main") {
            webview.close_devtools();
        }
    });
    result.into()
}

fn passwords_match(stored: &str, supplied: &str) -> bool {
    if stored.len() != supplied.len() {
        return false;
    }
    let difference = stored
        .bytes()
        .zip(supplied.bytes())
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        });
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locked_session_requires_valid_password_to_clear() {
        let state = AppLock::default();
        state.0.store(true, Ordering::SeqCst);
        assert!(state.locked());
        assert!(!state.unlock("correct horse", "correct house"));
        assert!(state.locked());
        assert!(state.unlock("correct horse", "correct horse"));
        assert!(!state.locked());
    }

    #[test]
    fn compares_full_passwords() {
        assert!(passwords_match("correct horse", "correct horse"));
        assert!(!passwords_match("correct horse", "correct"));
        assert!(!passwords_match("correct horse", "correct house"));
    }

    #[test]
    fn rejects_short_password() {
        assert!(matches!(
            validate("123"),
            Err(vault::Error::TooShort("app lock password", 4))
        ));
    }
}
