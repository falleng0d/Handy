use crate::settings::AutoSubmitKey;
use karabiner_input::{HelperStatus, RegistrationStatus, SubmitKey, TypeOptions};
use std::time::Duration;

pub fn ensure_registered() -> Result<(), String> {
    match karabiner_input::helper_status() {
        HelperStatus::Ready => return Ok(()),
        HelperStatus::Rejected(error) => {
            return Err(format!("Karabiner input helper rejected Handy: {error}"));
        }
        HelperStatus::NotRunning => {}
    }

    match karabiner_input::helper_registration_status().map_err(|e| e.to_string())? {
        RegistrationStatus::Enabled => Ok(()),
        RegistrationStatus::RequiresApproval => Err(
            "Karabiner input helper requires approval in System Settings > General > Login Items"
                .into(),
        ),
        RegistrationStatus::NotRegistered => {
            karabiner_input::register_helper().map_err(|e| e.to_string())?;
            match karabiner_input::helper_registration_status().map_err(|e| e.to_string())? {
                RegistrationStatus::Enabled => Ok(()),
                _ => Err(
                    "Karabiner input helper was registered; approve it in System Settings > General > Login Items"
                        .into(),
                ),
            }
        }
        RegistrationStatus::NotFound => {
            Err(
                "Karabiner input helper is unavailable; locally signed builds require `bun run install:karabiner-helper-local`"
                    .into(),
            )
        }
    }
}

pub fn type_text(
    text: &str,
    auto_submit: bool,
    auto_submit_key: AutoSubmitKey,
) -> Result<(), String> {
    ensure_registered()?;

    let submit = auto_submit.then_some(match auto_submit_key {
        AutoSubmitKey::Enter => SubmitKey::Enter,
        AutoSubmitKey::CtrlEnter => SubmitKey::ControlEnter,
        AutoSubmitKey::CmdEnter => SubmitKey::CommandEnter,
    });

    karabiner_input::type_text(
        text,
        TypeOptions {
            key_down: Duration::from_millis(1),
            inter_key: Duration::from_millis(1),
            submit,
        },
    )
    .map_err(|e| e.to_string())
}
