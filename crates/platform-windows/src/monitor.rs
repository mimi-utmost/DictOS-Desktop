//! DPI e área de trabalho dos monitores.
//!
//! O daemon roda como Per-Monitor V2 DPI-aware: todas as coordenadas que
//! trocamos com o Windows (âncora UIA, cursor, work area) e com o Slint
//! (`PhysicalPosition`) são pixels físicos reais, consistentes mesmo com
//! monitores em escalas diferentes.

use dictos_selection::ScreenRect;

/// Marca o processo como Per-Monitor V2 DPI-aware. Deve ser chamado no
/// início do daemon, antes de criar qualquer janela. Falhas são ignoradas
/// (ex.: awareness já definido por manifesto).
#[cfg(windows)]
pub fn set_per_monitor_v2_awareness() {
    use windows::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

/// Área de trabalho (work area: tela menos barra de tarefas) do monitor que
/// contém o retângulo dado — ou o monitor do cursor, se `None`.
#[cfg(windows)]
pub fn work_area_containing(rect: Option<ScreenRect>) -> ScreenRect {
    use windows::Win32::Foundation::{POINT, RECT};
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MonitorFromRect, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    unsafe {
        let monitor = match rect {
            Some(r) => MonitorFromRect(
                &RECT {
                    left: r.x,
                    top: r.y,
                    right: r.x + r.width,
                    bottom: r.y + r.height,
                },
                MONITOR_DEFAULTTONEAREST,
            ),
            None => {
                let mut point = POINT::default();
                let _ = GetCursorPos(&mut point);
                MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST)
            }
        };

        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            let w = info.rcWork;
            ScreenRect {
                x: w.left,
                y: w.top,
                width: w.right - w.left,
                height: w.bottom - w.top,
            }
        } else {
            // Fallback: tela virtual inteira.
            ScreenRect {
                x: 0,
                y: 0,
                width: i32::MAX / 2,
                height: i32::MAX / 2,
            }
        }
    }
}
