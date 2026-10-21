//! Política de captura por aplicativo ("blocklist").
//!
//! Alguns aplicativos reagem mal ao fallback de Ctrl+C simulado: em
//! terminais ele interrompe o processo em foreground, no Excel ele copia a
//! célula inteira. Para esses apps o fallback é desligado e a captura
//! depende só de UIA; se UIA falhar, o usuário recebe uma mensagem clara
//! em vez de um efeito colateral invisível.

/// O que pode ser usado para capturar a seleção do aplicativo em foreground.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapturePolicy {
    /// Permite captura via UI Automation.
    pub allow_uia: bool,
    /// Permite o fallback de Ctrl+C simulado (com backup/restore).
    pub allow_clipboard_fallback: bool,
    /// A janela em foreground pertence ao próprio DictOS (captura ignorada).
    pub is_own_process: bool,
}

/// Política padrão: UIA + fallback de clipboard.
pub const POLICY_FULL: CapturePolicy = CapturePolicy {
    allow_uia: true,
    allow_clipboard_fallback: true,
    is_own_process: false,
};

/// Política sem fallback: Ctrl+C simulado é perigoso ou inútil aqui.
const POLICY_NO_CLIPBOARD_FALLBACK: CapturePolicy = CapturePolicy {
    allow_uia: true,
    allow_clipboard_fallback: false,
    is_own_process: false,
};

/// Política para o próprio processo: nunca capturar de si mesmo.
const POLICY_SELF: CapturePolicy = CapturePolicy {
    allow_uia: false,
    allow_clipboard_fallback: false,
    is_own_process: true,
};

/// Resolve a política a partir da classe da janela em foreground.
///
/// Função pura: testável sem Windows.
pub fn policy_for_class(class_name: &str) -> CapturePolicy {
    match class_name {
        // Terminais: Ctrl+C interrompe o processo em foreground.
        "ConsoleWindowClass" | "CASCADIA_HOSTING_WINDOW_CLASS" | "mintty" => {
            POLICY_NO_CLIPBOARD_FALLBACK
        }
        // Excel: Ctrl+C copia a célula inteira, não o texto (SPEC: reage mal).
        "XLMAIN" => POLICY_NO_CLIPBOARD_FALLBACK,
        _ => POLICY_FULL,
    }
}

/// Política do aplicativo atualmente em foreground.
#[cfg(windows)]
pub fn foreground_policy() -> CapturePolicy {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetWindowThreadProcessId,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return POLICY_FULL;
        }

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == std::process::id() {
            return POLICY_SELF;
        }

        let mut buf = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut buf);
        if len <= 0 {
            return POLICY_FULL;
        }
        let class_name = String::from_utf16_lossy(&buf[..len as usize]);
        policy_for_class(&class_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminais_nao_recebem_ctrl_c() {
        for class in ["ConsoleWindowClass", "CASCADIA_HOSTING_WINDOW_CLASS", "mintty"] {
            let policy = policy_for_class(class);
            assert!(policy.allow_uia, "{class} deveria permitir UIA");
            assert!(!policy.allow_clipboard_fallback, "{class} não deveria receber Ctrl+C");
            assert!(!policy.is_own_process);
        }
    }

    #[test]
    fn excel_nao_recebe_ctrl_c() {
        let policy = policy_for_class("XLMAIN");
        assert!(policy.allow_uia);
        assert!(!policy.allow_clipboard_fallback);
    }

    #[test]
    fn apps_comuns_tem_politica_completa() {
        for class in ["Notepad", "Chrome_WidgetWin_1", "CabinetWClass", ""] {
            assert_eq!(policy_for_class(class), POLICY_FULL, "{class}");
        }
    }
}
