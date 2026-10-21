//! Popup de consulta do DictOS.
//!
//! O modelo de view ([`LookupView`]) e o trait [`Popup`] são independentes de
//! toolkit; a implementação atual usa Slint (licença royalty-free 2.0, com a
//! atribuição "Built with Slint" exibida no rodapé do popup).

use dictos_core::LookupResult;
use dictos_selection::ScreenRect;

pub mod placement;

pub use slint;

slint::include_modules!();

/// Contexto de posicionamento do popup, em pixels físicos.
#[derive(Debug, Clone, Copy)]
pub struct PlacementContext {
    /// Retângulo da seleção no aplicativo de origem (âncora).
    pub anchor: ScreenRect,
    /// Work area (tela menos barra de tarefas) do monitor da âncora.
    pub work_area: ScreenRect,
}

/// Uma acepção formatada para exibição.
#[derive(Debug, Clone)]
pub struct SenseView {
    pub pos: String,
    pub definition: String,
    pub examples: Vec<String>,
}

/// Resultado de consulta pronto para exibição, independente de toolkit.
#[derive(Debug, Clone)]
pub struct LookupView {
    pub word: String,
    pub diacritic_fallback: bool,
    pub senses: Vec<SenseView>,
    pub translations: Vec<(String, String)>,
}

impl From<&LookupResult> for LookupView {
    fn from(result: &LookupResult) -> Self {
        let entry = &result.entry;
        LookupView {
            word: entry.word.clone(),
            diacritic_fallback: result.diacritic_fallback,
            senses: entry
                .senses
                .iter()
                .map(|s| SenseView {
                    pos: s.pos.clone(),
                    definition: s.definition.clone(),
                    examples: s.examples.clone(),
                })
                .collect(),
            translations: entry
                .translations
                .iter()
                .map(|t| (t.lang.clone(), t.word.clone()))
                .collect(),
        }
    }
}

/// Interface de popup: cada plataforma tem sua implementação.
pub trait Popup {
    /// Exibe o resultado de uma consulta.
    fn show_lookup(&self, view: &LookupView, placement: Option<&PlacementContext>);
    /// Exibe uma mensagem (palavra desconhecida, falha de captura etc.).
    fn show_message(&self, title: &str, message: &str, placement: Option<&PlacementContext>);
    /// Esconde o popup.
    fn hide(&self);
}

/// Formata o corpo textual do popup a partir do view model.
pub fn format_body(view: &LookupView) -> String {
    let mut body = String::new();
    for (i, sense) in view.senses.iter().enumerate() {
        body.push_str(&format!("{}. ({})\n{}\n", i + 1, sense.pos, sense.definition));
        for example in &sense.examples {
            body.push_str(&format!("   • {example}\n"));
        }
        body.push('\n');
    }
    if !view.translations.is_empty() {
        body.push_str("Traduções:\n");
        for (lang, word) in &view.translations {
            body.push_str(&format!("  {lang}: {word}\n"));
        }
    }
    body.trim_end().to_string()
}

/// Implementação Slint do popup.
pub struct SlintPopup {
    ui: DictosPopup,
    _click_timer: slint::Timer,
}

impl SlintPopup {
    /// Cria o popup oculto. Deve ser chamado na thread do event loop Slint
    /// (a thread principal).
    pub fn new() -> Result<Self, slint::PlatformError> {
        let ui = DictosPopup::new()?;

        let weak = ui.as_weak();
        ui.on_close(move || {
            if let Some(ui) = weak.upgrade() {
                let _ = ui.window().hide();
            }
        });

        // Esconde o popup quando o usuário clica fora dele ou pressiona Esc.
        // O popup é deliberadamente NOACTIVATE (não recebe foco nem teclas),
        // então a detecção é global: polling via GetAsyncKeyState. O polling
        // usa o bit menos significativo ("pressionada desde a última
        // chamada"), que não perde apertos curtos entre dois ticks — por
        // isso a consulta roda em TODO tick, mesmo com o popup escondido,
        // para manter o bit sempre fresco.
        let click_timer = slint::Timer::default();
        let weak = ui.as_weak();
        click_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(100),
            move || {
                let Some(ui) = weak.upgrade() else { return };
                let esc = pressed_since_last_poll(Key::Escape);
                let click = pressed_since_last_poll(Key::LeftButton);
                if ui.window().is_visible() && (esc || (click && cursor_outside(&ui))) {
                    let _ = ui.window().hide();
                }
            },
        );

        Ok(Self {
            ui,
            _click_timer: click_timer,
        })
    }

    /// Handle fraco para uso a partir de outras threads via
    /// `slint::invoke_from_event_loop`.
    pub fn as_weak(&self) -> slint::Weak<DictosPopup> {
        self.ui.as_weak()
    }
}

impl Popup for SlintPopup {
    fn show_lookup(&self, view: &LookupView, placement: Option<&PlacementContext>) {
        show_lookup_ui(&self.ui, view, placement);
    }

    fn show_message(&self, title: &str, message: &str, placement: Option<&PlacementContext>) {
        show_message_ui(&self.ui, title, message, placement);
    }

    fn hide(&self) {
        let _ = self.ui.window().hide();
    }
}

/// Exibe um resultado de consulta num componente já criado (usável de
/// qualquer lugar com acesso ao componente, ex.: dentro de
/// `slint::invoke_from_event_loop`).
pub fn show_lookup_ui(ui: &DictosPopup, view: &LookupView, placement: Option<&PlacementContext>) {
    let note = if view.diacritic_fallback {
        "encontrado por busca sem acentos"
    } else {
        ""
    };
    present(ui, &view.word, format_body(view), note, placement);
}

/// Exibe uma mensagem (palavra desconhecida, falha de captura etc.).
pub fn show_message_ui(
    ui: &DictosPopup,
    title: &str,
    message: &str,
    placement: Option<&PlacementContext>,
) {
    present(ui, title, message.to_string(), "", placement);
}

/// Preenche o conteúdo, posiciona (junto à âncora ou ao cursor) e mostra
/// sem roubar o foco do aplicativo de origem.
fn present(
    ui: &DictosPopup,
    word: &str,
    body: String,
    note: &str,
    placement: Option<&PlacementContext>,
) {
    ui.set_word(word.into());
    ui.set_body(body.into());
    ui.set_note(note.into());

    let size = ui.window().size();
    let size = (size.width as i32, size.height as i32);
    let (x, y) = match placement {
        Some(ctx) => placement::place_popup(ctx.anchor, size, ctx.work_area),
        None => cursor_position(size),
    };
    ui.window().set_position(slint::PhysicalPosition::new(x, y));
    show_without_focus(ui);
}

/// Mostra a janela sem roubar o foco: marca o HWND como WS_EX_NOACTIVATE e,
/// se o show() do winit ativar a janela mesmo assim, devolve o foreground
/// para a janela que estava ativa antes.
#[cfg(windows)]
fn show_without_focus(ui: &DictosPopup) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowLongPtrW, SetForegroundWindow, SetWindowLongPtrW,
        SetWindowPos, GWL_EXSTYLE, HWND_TOP, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
        SWP_NOSIZE, SWP_NOZORDER, WS_EX_NOACTIVATE,
    };

    unsafe {
        let previous = GetForegroundWindow();
        let _ = ui.window().show();

        let window_handle = ui.window().window_handle();
        let Ok(handle) = window_handle.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(win) = handle.as_raw() else {
            return;
        };
        let hwnd = HWND(win.hwnd.get() as *mut _);

        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_NOACTIVATE.0 as isize);
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );

        // Se mesmo com NOACTIVATE o foco veio para o popup, devolve.
        if !previous.0.is_null() && GetForegroundWindow() == hwnd {
            let _ = SetForegroundWindow(previous);
        }
    }
}

#[cfg(not(windows))]
fn show_without_focus(ui: &DictosPopup) {
    let _ = ui.window().show();
}

/// Posição de fallback quando não há âncora: abaixo e à direita do cursor,
/// limitado à tela primária.
#[cfg(windows)]
fn cursor_position(size: (i32, i32)) -> (i32, i32) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
    };

    let mut point = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut point);
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let x = (point.x + 12).min(screen_w - size.0 - 8).max(0);
        let y = (point.y + 16).min(screen_h - size.1 - 8).max(0);
        (x, y)
    }
}

#[cfg(not(windows))]
fn cursor_position(_size: (i32, i32)) -> (i32, i32) {
    (100, 100)
}

/// Teclas/botões monitorados pelo polling do popup.
#[cfg(windows)]
#[derive(Clone, Copy)]
enum Key {
    Escape,
    LeftButton,
}

/// Verdadeiro se a tecla foi pressionada desde a última chamada desta
/// função (bit menos significativo de GetAsyncKeyState). Não perde apertos
/// mais curtos que o intervalo do timer — desde que seja chamada em todo
/// tick, mesmo com o popup escondido.
#[cfg(windows)]
fn pressed_since_last_poll(key: Key) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON,
    };

    let vk = match key {
        Key::Escape => VK_ESCAPE.0 as i32,
        Key::LeftButton => VK_LBUTTON.0 as i32,
    };
    unsafe { (GetAsyncKeyState(vk) & 1) != 0 }
}

/// Verdadeiro quando o cursor está fora do retângulo do popup.
#[cfg(windows)]
fn cursor_outside(ui: &DictosPopup) -> bool {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT::default();
    unsafe {
        if GetCursorPos(&mut point).is_err() {
            return false;
        }
    }
    let pos = ui.window().position();
    let size = ui.window().size();
    let inside = point.x >= pos.x
        && point.x < pos.x + size.width as i32
        && point.y >= pos.y
        && point.y < pos.y + size.height as i32;
    !inside
}

#[cfg(not(windows))]
fn cursor_outside(_ui: &DictosPopup) -> bool {
    false
}

#[cfg(not(windows))]
#[derive(Clone, Copy)]
enum Key {
    Escape,
    LeftButton,
}

#[cfg(not(windows))]
fn pressed_since_last_poll(_key: Key) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_body_com_senses_e_traducoes() {
        let view = LookupView {
            word: "apple".into(),
            diacritic_fallback: false,
            senses: vec![SenseView {
                pos: "noun".into(),
                definition: "A round fruit.".into(),
                examples: vec!["She ate an apple.".into()],
            }],
            translations: vec![("pt".into(), "maçã".into())],
        };
        let body = format_body(&view);
        assert!(body.contains("1. (noun)"));
        assert!(body.contains("A round fruit."));
        assert!(body.contains("• She ate an apple."));
        assert!(body.contains("pt: maçã"));
    }

    #[test]
    fn lookup_view_from_result() {
        use dictos_core::{DictionaryEntry, Sense, Translation};
        let result = LookupResult {
            entry: DictionaryEntry {
                word: "run".into(),
                lang: "en".into(),
                senses: vec![Sense {
                    pos: "verb".into(),
                    definition: "To move swiftly.".into(),
                    examples: vec![],
                }],
                translations: vec![Translation {
                    lang: "pt".into(),
                    word: "correr".into(),
                }],
            },
            diacritic_fallback: false,
        };
        let view = LookupView::from(&result);
        assert_eq!(view.word, "run");
        assert_eq!(view.senses.len(), 1);
        assert_eq!(view.translations, vec![("pt".to_string(), "correr".to_string())]);
    }
}
