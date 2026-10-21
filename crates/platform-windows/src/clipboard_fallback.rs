//! Fallback de captura via área de transferência, com preservação total.
//!
//! Pipeline: backup de todos os formatos → Ctrl+C simulado → espera o
//! clipboard mudar → lê o texto → restaura o backup e **verifica** a
//! restauração. Se a restauração falhar, a consulta é abortada: o conteúdo do
//! usuário nunca é sobrescrito silenciosamente.

use std::thread::sleep;
use std::time::{Duration, Instant};

use dictos_selection::{CapturedSelection, SelectionError};

use windows::core::PWSTR;
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
    GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_C, VK_CONTROL,
    VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};

const CF_UNICODETEXT: u32 = 13;
const CF_OWNERDISPLAY: u32 = 0x80;
const CF_PRIVATEFIRST: u32 = 0x200;
const CF_PRIVATELAST: u32 = 0x2FF;

/// Formatos que o Windows sintetiza a partir de outros — restaurar os
/// "pais" já os regenera; copiá-los manualmente pode duplicar/corromper.
const SYNTHESIZED: [u32; 17] = [
    1,  // CF_TEXT
    2,  // CF_BITMAP
    3,  // CF_METAFILEPICT
    4,  // CF_SYLK
    5,  // CF_DIF
    6,  // CF_TIFF
    7,  // CF_OEMTEXT
    8,  // CF_DIB
    10, // CF_PENDATA
    11, // CF_RIFF
    12, // CF_WAVE
    14, // CF_ENHMETAFILE
    16, // CF_LOCALE
    17, // CF_DIBV5
    0x81, // CF_DSPTEXT
    0x82, // CF_DSPBITMAP
    0x83, // CF_DSPMETAFILEPICT
];

/// Cópia dos dados da área de transferência antes do Ctrl+C simulado.
pub struct ClipboardBackup {
    formats: Vec<(u32, Vec<u8>)>,
}

/// Executa o fallback completo e devolve o texto capturado.
///
/// Erros: `NoSelection` se o clipboard não mudou ou não trouxe texto;
/// `ClipboardRestoreFailed` se o conteúdo original não pôde ser restaurado
/// (nesse caso nenhum resultado é devolvido).
pub fn capture_via_clipboard() -> Result<CapturedSelection, SelectionError> {
    let backup = backup()?;
    let sequence_before = unsafe { GetClipboardSequenceNumber() };

    send_ctrl_c()?;

    if !wait_for_change(sequence_before, Duration::from_millis(500)) {
        return Err(SelectionError::NoSelection);
    }

    let text = read_unicode_text()?;
    restore(&backup)?;

    let text = text.ok_or(SelectionError::NoSelection)?;
    if text.trim().is_empty() {
        return Err(SelectionError::NoSelection);
    }
    Ok(CapturedSelection {
        text,
        anchor: None,
    })
}

fn should_skip(format: u32) -> bool {
    SYNTHESIZED.contains(&format)
        || format == CF_OWNERDISPLAY
        || (CF_PRIVATEFIRST..=CF_PRIVATELAST).contains(&format)
}

/// Abre o clipboard com algumas tentativas (outro app pode estar com ele aberto).
fn open_clipboard() -> Result<(), SelectionError> {
    for _ in 0..10 {
        if unsafe { OpenClipboard(None) }.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(10));
    }
    Err(SelectionError::CaptureFailed(
        "clipboard ocupado por outro aplicativo".into(),
    ))
}

/// Copia os dados de todos os formatos preserváveis do clipboard.
fn backup() -> Result<ClipboardBackup, SelectionError> {
    open_clipboard()?;
    let result = (|| {
        let mut formats = Vec::new();
        let mut format = 0;
        loop {
            format = unsafe { EnumClipboardFormats(format) };
            if format == 0 {
                break;
            }
            if should_skip(format) {
                continue;
            }
            let handle = match unsafe { GetClipboardData(format) } {
                Ok(h) => h,
                Err(_) => continue,
            };
            if handle.0.is_null() {
                // delayed rendering: o dado não existe materializado; não dá
                // para preservar — pulamos em vez de corromper.
                continue;
            }
            let hglobal = HGLOBAL(handle.0.cast());
            let size = unsafe { GlobalSize(hglobal) };
            let ptr = unsafe { GlobalLock(hglobal) };
            if ptr.is_null() || size == 0 {
                continue;
            }
            let data = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), size) }.to_vec();
            unsafe {
                let _ = GlobalUnlock(hglobal);
            }
            formats.push((format, data));
        }
        Ok(ClipboardBackup { formats })
    })();
    unsafe {
        let _ = CloseClipboard();
    }
    result
}

/// Envia Ctrl+C, soltando antes os modificadores do nosso próprio hotkey
/// (para não virar Ctrl+Shift+C no aplicativo de origem).
fn send_ctrl_c() -> Result<(), SelectionError> {
    let modifiers = [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN];

    let mut inputs: Vec<INPUT> = modifiers
        .iter()
        .map(|&vk| key_event(vk, true))
        .collect();
    inputs.push(key_event(VK_CONTROL, false));
    inputs.push(key_event(VK_C, false));
    inputs.push(key_event(VK_C, true));
    inputs.push(key_event(VK_CONTROL, true));

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        return Err(SelectionError::CaptureFailed(
            "SendInput não entregou todos os eventos".into(),
        ));
    }
    Ok(())
}

fn key_event(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if key_up {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Espera o número de sequência do clipboard mudar (indica novo conteúdo).
fn wait_for_change(before: u32, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if unsafe { GetClipboardSequenceNumber() } != before {
            return true;
        }
        sleep(Duration::from_millis(10));
    }
    false
}

/// Lê o conteúdo CF_UNICODETEXT atual do clipboard.
fn read_unicode_text() -> Result<Option<String>, SelectionError> {
    open_clipboard()?;
    let result = (|| {
        if unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) }.is_err() {
            return Ok(None);
        }
        let handle = unsafe { GetClipboardData(CF_UNICODETEXT) }
            .map_err(|e| SelectionError::CaptureFailed(format!("GetClipboardData: {e}")))?;
        if handle.0.is_null() {
            return Ok(None);
        }
        let hglobal = HGLOBAL(handle.0.cast());
        let ptr = unsafe { GlobalLock(hglobal) };
        if ptr.is_null() {
            return Ok(None);
        }
        let text = unsafe { PWSTR(ptr.cast()).to_string() }
            .map_err(|e| SelectionError::CaptureFailed(format!("texto inválido: {e}")))?;
        unsafe {
            let _ = GlobalUnlock(hglobal);
        }
        Ok(Some(text.trim_end_matches('\0').to_string()))
    })();
    unsafe {
        let _ = CloseClipboard();
    }
    result
}

/// Restaura o backup no clipboard e verifica o conteúdo textual.
fn restore(backup: &ClipboardBackup) -> Result<(), SelectionError> {
    open_clipboard().map_err(|_| SelectionError::ClipboardRestoreFailed)?;
    let result = (|| {
        unsafe {
            EmptyClipboard().map_err(|_| SelectionError::ClipboardRestoreFailed)?;
        }
        for (format, data) in &backup.formats {
            let hglobal = unsafe { GlobalAlloc(GMEM_MOVEABLE, data.len()) }
                .map_err(|_| SelectionError::ClipboardRestoreFailed)?;
            let ptr = unsafe { GlobalLock(hglobal) };
            if ptr.is_null() {
                unsafe {
                    let _ = GlobalFree(Some(hglobal));
                }
                return Err(SelectionError::ClipboardRestoreFailed);
            }
            unsafe {
                std::ptr::copy_nonoverlapping(data.as_ptr(), ptr.cast::<u8>(), data.len());
                let _ = GlobalUnlock(hglobal);
            }
            // Em caso de sucesso o sistema passa a ser dono do HGLOBAL.
            if unsafe { SetClipboardData(*format, Some(HANDLE(hglobal.0.cast()))) }.is_err() {
                unsafe {
                    let _ = GlobalFree(Some(hglobal));
                }
                return Err(SelectionError::ClipboardRestoreFailed);
            }
        }
        Ok(())
    })();
    unsafe {
        let _ = CloseClipboard();
    }
    result?;

    // Verificação: o texto restaurado precisa bater com o backup.
    let expected = backup
        .formats
        .iter()
        .find(|(format, _)| *format == CF_UNICODETEXT)
        .map(|(_, data)| data.clone());
    if let Some(expected) = expected {
        let restored = read_unicode_text()
            .map_err(|_| SelectionError::ClipboardRestoreFailed)?
            .ok_or(SelectionError::ClipboardRestoreFailed)?;
        let restored_utf16: Vec<u8> = restored
            .encode_utf16()
            .flat_map(|unit| unit.to_le_bytes())
            .collect();
        let mut expected_trimmed = Vec::new();
        for pair in expected.chunks_exact(2) {
            if pair[0] == 0 && pair[1] == 0 {
                break;
            }
            expected_trimmed.extend_from_slice(pair);
        }
        if restored_utf16 != expected_trimmed {
            return Err(SelectionError::ClipboardRestoreFailed);
        }
    }
    Ok(())
}

/// Escreve texto no clipboard (usado nos testes e disponível para diagnóstico).
pub fn set_unicode_text_for_test(text: &str) -> Result<(), SelectionError> {
    let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes: Vec<u8> = utf16.iter().flat_map(|u| u.to_le_bytes()).collect();

    open_clipboard()?;
    let result = (|| {
        unsafe {
            EmptyClipboard().map_err(|e| SelectionError::CaptureFailed(format!("{e}")))?;
        }
        let hglobal = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) }
            .map_err(|e| SelectionError::CaptureFailed(format!("{e}")))?;
        let ptr = unsafe { GlobalLock(hglobal) };
        if ptr.is_null() {
            unsafe {
                let _ = GlobalFree(Some(hglobal));
            }
            return Err(SelectionError::CaptureFailed("GlobalLock falhou".into()));
        }
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.cast::<u8>(), bytes.len());
            let _ = GlobalUnlock(hglobal);
            SetClipboardData(CF_UNICODETEXT, Some(HANDLE(hglobal.0.cast())))
                .map_err(|e| SelectionError::CaptureFailed(format!("{e}")))?;
        }
        Ok(())
    })();
    unsafe {
        let _ = CloseClipboard();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Teste real no Windows: backup → sobrescreve → restore → compara.
    /// Usa o clipboard de verdade; roda serializado por ser estado global.
    #[test]
    fn backup_e_restore_preserva_texto() {
        set_unicode_text_for_test("dictos-teste-original-123").unwrap();
        let backup = backup().unwrap();

        set_unicode_text_for_test("conteudo-sobrescrito").unwrap();
        assert_eq!(
            read_unicode_text().unwrap().as_deref(),
            Some("conteudo-sobrescrito")
        );

        restore(&backup).unwrap();
        assert_eq!(
            read_unicode_text().unwrap().as_deref(),
            Some("dictos-teste-original-123")
        );
    }

    #[test]
    fn formatos_sintetizados_e_privados_sao_ignorados() {
        assert!(should_skip(1)); // CF_TEXT
        assert!(should_skip(8)); // CF_DIB
        assert!(should_skip(CF_OWNERDISPLAY));
        assert!(should_skip(CF_PRIVATEFIRST + 10));
        assert!(!should_skip(CF_UNICODETEXT));
    }
}
