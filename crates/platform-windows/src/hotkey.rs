//! Atalho global via crate `global-hotkey` (RegisterHotKey no Windows).

use dictos_selection::SelectionError;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyEventReceiver, GlobalHotKeyManager};

pub use global_hotkey::HotKeyState;

/// Analisa um hotkey no formato "Ctrl+Shift+D".
///
/// Modificadores aceitos: ctrl, shift, alt, win/super. Tecla: letra A-Z,
/// dígito 0-9 ou F1-F12.
pub fn parse_hotkey(input: &str) -> Result<HotKey, SelectionError> {
    let mut modifiers = Modifiers::empty();
    let mut key: Option<Code> = None;

    for part in input.split('+').map(|p| p.trim()).filter(|p| !p.is_empty()) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" => modifiers |= Modifiers::ALT,
            "win" | "super" | "meta" => modifiers |= Modifiers::SUPER,
            name => {
                if key.is_some() {
                    return Err(SelectionError::CaptureFailed(format!(
                        "hotkey com mais de uma tecla: '{input}'"
                    )));
                }
                key = Some(parse_key(name).ok_or_else(|| {
                    SelectionError::CaptureFailed(format!("tecla desconhecida: '{part}'"))
                })?);
            }
        }
    }

    let code = key.ok_or_else(|| {
        SelectionError::CaptureFailed(format!("hotkey sem tecla principal: '{input}'"))
    })?;
    Ok(HotKey::new(Some(modifiers), code))
}

fn parse_key(name: &str) -> Option<Code> {
    let upper = name.to_ascii_uppercase();
    match upper.as_str() {
        "SPACE" => return Some(Code::Space),
        "TAB" => return Some(Code::Tab),
        "ENTER" | "RETURN" => return Some(Code::Enter),
        "ESCAPE" | "ESC" => return Some(Code::Escape),
        _ => {}
    }
    if let Some(f) = upper.strip_prefix('F') {
        if let Ok(n) = f.parse::<u32>() {
            return match n {
                1 => Some(Code::F1),
                2 => Some(Code::F2),
                3 => Some(Code::F3),
                4 => Some(Code::F4),
                5 => Some(Code::F5),
                6 => Some(Code::F6),
                7 => Some(Code::F7),
                8 => Some(Code::F8),
                9 => Some(Code::F9),
                10 => Some(Code::F10),
                11 => Some(Code::F11),
                12 => Some(Code::F12),
                _ => None,
            };
        }
    }
    if upper.len() == 1 {
        let c = upper.chars().next()?;
        if c.is_ascii_uppercase() {
            // Code::KeyA..KeyZ são consecutivos nessa faixa no enum.
            return Some(match c {
                'A' => Code::KeyA,
                'B' => Code::KeyB,
                'C' => Code::KeyC,
                'D' => Code::KeyD,
                'E' => Code::KeyE,
                'F' => Code::KeyF,
                'G' => Code::KeyG,
                'H' => Code::KeyH,
                'I' => Code::KeyI,
                'J' => Code::KeyJ,
                'K' => Code::KeyK,
                'L' => Code::KeyL,
                'M' => Code::KeyM,
                'N' => Code::KeyN,
                'O' => Code::KeyO,
                'P' => Code::KeyP,
                'Q' => Code::KeyQ,
                'R' => Code::KeyR,
                'S' => Code::KeyS,
                'T' => Code::KeyT,
                'U' => Code::KeyU,
                'V' => Code::KeyV,
                'W' => Code::KeyW,
                'X' => Code::KeyX,
                'Y' => Code::KeyY,
                'Z' => Code::KeyZ,
                _ => return None,
            });
        }
        if c.is_ascii_digit() {
            return Some(match c {
                '0' => Code::Digit0,
                '1' => Code::Digit1,
                '2' => Code::Digit2,
                '3' => Code::Digit3,
                '4' => Code::Digit4,
                '5' => Code::Digit5,
                '6' => Code::Digit6,
                '7' => Code::Digit7,
                '8' => Code::Digit8,
                '9' => Code::Digit9,
                _ => return None,
            });
        }
    }
    None
}

/// Registra o hotkey global e devolve o manager (mantém o registro vivo) e o
/// receiver de eventos para o loop do daemon. Erro claro se outro app já
/// registrou a mesma combinação.
pub fn register(hotkey: &HotKey) -> Result<(GlobalHotKeyManager, GlobalHotKeyEventReceiver), SelectionError> {
    let manager = GlobalHotKeyManager::new()
        .map_err(|e| SelectionError::CaptureFailed(format!("falha ao iniciar hotkeys: {e}")))?;
    manager
        .register(*hotkey)
        .map_err(|e| {
            SelectionError::CaptureFailed(format!(
                "hotkey já registrado por outro aplicativo ou inválido: {e}"
            ))
        })?;
    Ok((manager, GlobalHotKeyEvent::receiver().clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ctrl_shift_d() {
        let hotkey = parse_hotkey("Ctrl+Shift+D").unwrap();
        assert_eq!(hotkey.mods, Modifiers::CONTROL | Modifiers::SHIFT);
        assert_eq!(hotkey.key, Code::KeyD);
    }

    #[test]
    fn parse_case_insensitive_e_espacos() {
        let hotkey = parse_hotkey(" alt + f2 ").unwrap();
        assert_eq!(hotkey.mods, Modifiers::ALT);
        assert_eq!(hotkey.key, Code::F2);
    }

    #[test]
    fn rejeita_sem_tecla() {
        assert!(parse_hotkey("Ctrl+Shift").is_err());
    }

    #[test]
    fn rejeita_duas_teclas() {
        assert!(parse_hotkey("Ctrl+A+B").is_err());
    }

    #[test]
    fn parse_digito() {
        let hotkey = parse_hotkey("Win+1").unwrap();
        assert_eq!(hotkey.mods, Modifiers::SUPER);
        assert_eq!(hotkey.key, Code::Digit1);
    }
}
