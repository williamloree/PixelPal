//! Couche Windows : petites fenêtres transparentes (« layered ») toujours au premier plan,
//! énumération des écrans, icône de la zone de notification.

use crate::monde::Ecran;
use crate::toile::Toile;
use std::ffi::c_void;
use std::sync::Mutex;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{BOOL, w};

/// Clics reçus par les fenêtres (handle de fenêtre, position dans la fenêtre), lus par la boucle principale.
pub static CLICS: Mutex<Vec<(isize, i32, i32)>> = Mutex::new(Vec::new());
pub static ECRANS_CHANGES: Mutex<bool> = Mutex::new(false);
pub static QUITTER: Mutex<bool> = Mutex::new(false);
const WM_ICONE: u32 = WM_APP + 1;

/// Menu du clic droit sur l'icône de la zone de notification.
unsafe fn menu_icone(h: HWND) {
    unsafe {
        let Ok(m) = CreatePopupMenu() else { return };
        let _ = AppendMenuW(m, MF_STRING, 1, w!("Ouvrir le panneau"));
        let _ = AppendMenuW(m, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(m, MF_STRING, 2, w!("Quitter PixelPal"));
        let mut p = POINT::default();
        let _ = GetCursorPos(&mut p);
        let _ = SetForegroundWindow(h); // sinon le menu ne se ferme pas en cliquant ailleurs
        let choix = TrackPopupMenu(m, TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN, p.x, p.y, None, h, None);
        let _ = DestroyMenu(m);
        match choix.0 {
            1 => CLICS.lock().unwrap().push((h.0 as isize, 0, 0)),
            2 => *QUITTER.lock().unwrap() = true,
            _ => {}
        }
    }
}

fn wstr(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }

unsafe extern "system" fn proc_fenetre(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_LBUTTONDOWN => {
            let (x, y) = ((lp.0 & 0xFFFF) as i16 as i32, ((lp.0 >> 16) & 0xFFFF) as i16 as i32);
            CLICS.lock().unwrap().push((h.0 as isize, x, y));
            unsafe { windows::Win32::UI::Input::KeyboardAndMouse::SetCapture(h); } // on garde la souris pendant qu'il est tenu
            LRESULT(0)
        }
        WM_LBUTTONUP => { unsafe { let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture(); } LRESULT(0) }
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_SETCURSOR => {
            unsafe { SetCursor(LoadCursorW(None, IDC_HAND).ok()); }
            LRESULT(1)
        }
        WM_DISPLAYCHANGE => { *ECRANS_CHANGES.lock().unwrap() = true; LRESULT(0) }
        WM_ICONE => {
            let e = (lp.0 & 0xFFFF) as u32;
            if e == WM_LBUTTONUP { CLICS.lock().unwrap().push((h.0 as isize, 0, 0)); }
            if e == WM_RBUTTONUP { unsafe { menu_icone(h) }; }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
    }
}

/// Prend le verrou « une seule instance par profil » (libéré à la fermeture du processus).
/// `attendre` : après une mise à jour, on attend que l'ancienne version ait fini de se fermer.
pub fn verrou_instance(profil: &Option<String>, attendre: bool) -> bool {
    use windows::Win32::System::Threading::{CreateMutexW, WaitForSingleObject};
    let nom = wstr(&format!("Local\\PixelPal-{}", profil.as_deref().unwrap_or("principal")));
    unsafe {
        let Ok(h) = CreateMutexW(None, true, windows::core::PCWSTR(nom.as_ptr())) else { return true };
        if GetLastError() != ERROR_ALREADY_EXISTS { return true; } // le verrou reste à nous jusqu'à la fin du processus
        if attendre {
            let r = WaitForSingleObject(h, 15_000);
            if r == WAIT_OBJECT_0 || r == WAIT_ABANDONED { return true; }
        }
        let _ = CloseHandle(h);
        false
    }
}

pub fn initialiser() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let inst = GetModuleHandleW(None).unwrap();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(proc_fenetre),
            hInstance: inst.into(),
            lpszClassName: w!("PixelPalCalque"),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            ..Default::default()
        };
        RegisterClassW(&wc);
    }
}

/// Une fenêtre transparente qui affiche une toile.
pub struct Calque {
    pub hwnd: HWND,
    w: i32,
    h: i32,
    dc: HDC,
    bmp: HBITMAP,
    ancien: HGDIOBJ,
    bits: *mut u8,
    visible: bool,
    cle: String,
}

impl Calque {
    pub fn new(cliquable: bool) -> Calque {
        unsafe {
            let mut ex = WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
            if !cliquable { ex |= WS_EX_TRANSPARENT; }
            let hwnd = CreateWindowExW(ex, w!("PixelPalCalque"), w!("PixelPal"), WS_POPUP, 0, 0, 1, 1, None, None,
                Some(GetModuleHandleW(None).unwrap().into()), None).expect("CreateWindowExW");
            let ecran = GetDC(None);
            let dc = CreateCompatibleDC(Some(ecran));
            ReleaseDC(None, ecran);
            Calque { hwnd, w: 0, h: 0, dc, bmp: HBITMAP::default(), ancien: HGDIOBJ::default(), bits: std::ptr::null_mut(), visible: false, cle: String::new() }
        }
    }

    fn preparer(&mut self, w: i32, h: i32) {
        if w == self.w && h == self.h && !self.bits.is_null() { return; }
        unsafe {
            if !self.bmp.is_invalid() { SelectObject(self.dc, self.ancien); let _ = DeleteObject(self.bmp.into()); }
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER { biSize: size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, ..Default::default() },
                ..Default::default()
            };
            let mut bits: *mut c_void = std::ptr::null_mut();
            self.bmp = CreateDIBSection(Some(self.dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0).expect("CreateDIBSection");
            self.ancien = SelectObject(self.dc, self.bmp.into());
            self.bits = bits as *mut u8;
            self.w = w;
            self.h = h;
        }
    }

    fn pixels(&mut self) -> &mut [u8] { unsafe { std::slice::from_raw_parts_mut(self.bits, (self.w * self.h * 4) as usize) } }

    fn pousser(&mut self, x: i32, y: i32) {
        unsafe {
            let ecran = GetDC(None);
            let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
            let _ = UpdateLayeredWindow(self.hwnd, Some(ecran), Some(&POINT { x, y }), Some(&SIZE { cx: self.w, cy: self.h }), Some(self.dc),
                Some(&POINT { x: 0, y: 0 }), COLORREF(0), Some(&blend), ULW_ALPHA);
            ReleaseDC(None, ecran);
            if !self.visible { let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE); self.visible = true; }
        }
    }
}

/// Ordre d'empilement. `devant` : au-dessus de tout (du haut vers le bas) ; `derriere` : au niveau du bureau,
/// sous toutes les fenêtres des applications (du haut vers le bas).
pub fn empiler(devant: &[HWND], derriere: &[HWND]) {
    unsafe {
        for h in devant.iter().rev() { let _ = SetWindowPos(*h, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE); }
        // HWND_BOTTOM retire l'état « toujours visible » ; le dernier placé finit tout en bas
        for h in derriere { let _ = SetWindowPos(*h, Some(HWND_BOTTOM), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE); }
    }
}

/// Bouton gauche de la souris enfoncé (même hors de nos fenêtres).
pub fn bouton_gauche() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
    unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0 }
}

/// Dessus visibles des fenêtres ouvertes, où le personnage peut marcher. La fenêtre active
/// (au premier plan) est exclue : il ne vient pas déranger l'appli qu'on utilise.
pub fn plateformes() -> Vec<crate::monde::Plateforme> {
    use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
    unsafe extern "system" fn cb(h: HWND, lp: LPARAM) -> BOOL {
        unsafe { (*(lp.0 as *mut Vec<HWND>)).push(h); }
        BOOL(1)
    }
    let mut toutes: Vec<HWND> = vec![];
    unsafe { let _ = EnumWindows(Some(cb), LPARAM(&mut toutes as *mut _ as isize)); }
    let active = unsafe { GetForegroundWindow() };
    // fenêtres « pleines », de la plus haute à la plus basse
    let mut pleines: Vec<(HWND, RECT, bool)> = vec![];
    for h in toutes {
        unsafe {
            if !IsWindowVisible(h).as_bool() || IsIconic(h).as_bool() { continue; }
            let mut masque: u32 = 0;
            let _ = DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut masque as *mut _ as *mut c_void, 4);
            if masque != 0 { continue; }
            let ex = GetWindowLongW(h, GWL_EXSTYLE) as u32;
            if ex & WS_EX_TOOLWINDOW.0 != 0 || ex & WS_EX_TRANSPARENT.0 != 0 { continue; }
            if GetWindowTextLengthW(h) == 0 { continue; }
            let mut classe = [0u16; 64];
            let n = GetClassNameW(h, &mut classe) as usize;
            let classe = String::from_utf16_lossy(&classe[..n]);
            if ["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd", "Windows.UI.Core.CoreWindow"].contains(&classe.as_str()) { continue; }
            let mut r = RECT::default();
            if DwmGetWindowAttribute(h, DWMWA_EXTENDED_FRAME_BOUNDS, &mut r as *mut _ as *mut c_void, size_of::<RECT>() as u32).is_err() {
                let _ = GetWindowRect(h, &mut r);
            }
            if r.right - r.left < 150 || r.bottom - r.top < 80 { continue; }
            let praticable = h != active && !IsZoomed(h).as_bool();
            pleines.push((h, r, praticable));
        }
    }
    let mut p = vec![];
    for (i, (h, r, praticable)) in pleines.iter().enumerate() {
        if !praticable { continue; }
        // on retire les morceaux du bord cachés par une fenêtre plus haute (ou sans place au-dessus)
        let mut morceaux = vec![(r.left, r.right)];
        for (_, o, _) in &pleines[..i] {
            if !(o.top < r.top + 2 && o.bottom > r.top - 12) { continue; }
            morceaux = morceaux.into_iter().flat_map(|(a, b)| {
                let mut v = vec![];
                if o.left > a { v.push((a, o.left.min(b))); }
                if o.right < b { v.push((o.right.max(a), b)); }
                v
            }).filter(|(a, b)| b - a > 0).collect();
        }
        for (a, b) in morceaux {
            if b - a >= 100 { p.push(crate::monde::Plateforme { hwnd: h.0 as isize, x0: a as f32, x1: b as f32, y: r.top as f32, gauche: r.left as f32 }); }
        }
    }
    p
}

/// Titre de la fenêtre au premier plan (ce qu'il espionne aux jumelles), hors bureau, barre des tâches et PixelPal.
pub fn titre_actif() -> Option<String> {
    unsafe {
        let h = GetForegroundWindow();
        if h.is_invalid() { return None; }
        let mut classe = [0u16; 64];
        let n = GetClassNameW(h, &mut classe) as usize;
        let classe = String::from_utf16_lossy(&classe[..n]);
        if ["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd", "PixelPalCalque"].contains(&classe.as_str()) { return None; }
        let mut buf = [0u16; 256];
        let n = GetWindowTextW(h, &mut buf).max(0) as usize;
        let t = String::from_utf16_lossy(&buf[..n]).trim().to_string();
        if t.is_empty() || t.starts_with("PixelPal") { None } else { Some(t) }
    }
}

impl Calque {

    /// Affiche la toile, coin haut-gauche en (x, y) dans l'écran virtuel.
    pub fn afficher(&mut self, t: &Toile, x: i32, y: i32) {
        self.preparer(t.w, t.h);
        t.vers_bgra_premul(self.pixels());
        self.pousser(x, y);
    }

    /// Bulle de texte (titre en gras + texte), la pointe centrée en bas sur (cx, bas).
    pub fn bulle(&mut self, titre: &str, texte: &str, indice: &str, cx: i32, bas: i32, k: f32) {
        let cle = format!("{titre}\u{1}{texte}\u{1}{indice}\u{1}{k}");
        if cle == self.cle && !self.bits.is_null() { return self.pousser(cx - self.w / 2, bas - self.h); }
        self.cle = cle;
        let u = (3.0 * k).round().max(1.0) as i32;
        let largeur_max = (300.0 * k) as i32;
        unsafe {
            let police = |taille: f32, gras: bool| CreateFontW((taille * k) as i32, 0, 0, 0, if gras { 700 } else { 400 }, 0, 0, 0,
                DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, (FF_SWISS.0 | VARIABLE_PITCH.0) as u32, w!("Segoe UI"));
            let (f_titre, f_texte, f_indice) = (police(15.0, true), police(15.0, false), police(12.0, false));
            let mesurer = |f: HFONT, s: &str| -> (i32, i32) {
                if s.is_empty() { return (0, 0); }
                let mut v: Vec<u16> = s.encode_utf16().collect();
                let mut r = RECT { left: 0, top: 0, right: largeur_max, bottom: 0 };
                let ancien = SelectObject(self.dc, f.into());
                DrawTextW(self.dc, &mut v, &mut r, DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX);
                SelectObject(self.dc, ancien);
                (r.right, r.bottom)
            };
            let (wt, ht) = mesurer(f_titre, titre);
            let (wx, hx) = mesurer(f_texte, texte);
            let (wi, hi) = mesurer(f_indice, indice);
            let marge = 4 * u;
            let interieur_w = wt.max(wx).max(wi).max(20 * u);
            let interieur_h = ht + hx + hi + if ht > 0 && hx > 0 { u } else { 0 } + if hi > 0 { u } else { 0 };
            let (bw, bh) = (interieur_w + 2 * marge, interieur_h + 2 * marge);
            let (w, h) = (bw, bh + 4 * u);

            // fond pixel art : contour sombre, coins coupés, pointe vers le bas
            let mut t = Toile::new(w, h);
            let (noir, creme, ombre) = (0xFF1D1520, 0xFFFBF3DF, 0xFFDCCFAF);
            t.rect(noir, u, 0, bw - 2 * u, bh); t.rect(noir, 0, u, bw, bh - 2 * u);
            t.rect(creme, 2 * u, u, bw - 4 * u, bh - 2 * u); t.rect(creme, u, 2 * u, bw - 2 * u, bh - 4 * u);
            t.rect(ombre, 2 * u, bh - 2 * u, bw - 4 * u, u);
            let px = bw / 2;
            t.rect(noir, px - 3 * u, bh - u, 6 * u, 2 * u); t.rect(creme, px - 2 * u, bh - u, 4 * u, u);
            t.rect(noir, px - 2 * u, bh + u, 4 * u, u); t.rect(creme, px - u, bh, 2 * u, u);
            t.rect(noir, px - u, bh + 2 * u, 2 * u, u);
            self.preparer(w, h);
            t.vers_bgra_premul(self.pixels());

            SetBkMode(self.dc, TRANSPARENT);
            let mut y = marge;
            let mut ecrire = |f: HFONT, s: &str, coul: u32, hauteur: i32| {
                if s.is_empty() { return; }
                let mut v: Vec<u16> = s.encode_utf16().collect();
                let mut r = RECT { left: marge, top: y, right: marge + interieur_w, bottom: y + hauteur };
                let ancien = SelectObject(self.dc, f.into());
                SetTextColor(self.dc, COLORREF(coul));
                DrawTextW(self.dc, &mut v, &mut r, DT_WORDBREAK | DT_NOPREFIX);
                SelectObject(self.dc, ancien);
                y += hauteur + u;
            };
            ecrire(f_titre, titre, 0x00_8E2620u32.swap_bgr(), ht);
            ecrire(f_texte, texte, 0x00_1D1520u32.swap_bgr(), hx);
            ecrire(f_indice, indice, 0x00_7A6A55u32.swap_bgr(), hi);
            for f in [f_titre, f_texte, f_indice] { let _ = DeleteObject(f.into()); }
        }
        // GDI remet l'alpha à 0 là où il écrit : on rend l'intérieur de la bulle opaque
        let (ww, bh) = (self.w, self.h - 4 * u);
        let p = self.pixels();
        for yy in u..bh - u {
            for xx in u..ww - u {
                p[((yy * ww + xx) * 4 + 3) as usize] = 255;
            }
        }
        self.pousser(cx - ww / 2, bas - self.h);
    }
}

trait Bgr { fn swap_bgr(self) -> u32; }
impl Bgr for u32 { fn swap_bgr(self) -> u32 { ((self & 0xFF) << 16) | (self & 0xFF00) | ((self >> 16) & 0xFF) } }

impl Drop for Calque {
    fn drop(&mut self) {
        unsafe {
            if !self.bmp.is_invalid() { SelectObject(self.dc, self.ancien); let _ = DeleteObject(self.bmp.into()); }
            let _ = DeleteDC(self.dc);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

/// Écrans tels que Windows les dispose (pixels physiques), et facteur d'échelle de l'écran principal.
pub fn ecrans() -> (Vec<Ecran>, f32) {
    unsafe extern "system" fn cb(m: HMONITOR, _: HDC, _: *mut RECT, lp: LPARAM) -> BOOL {
        let v = unsafe { &mut *(lp.0 as *mut Vec<(HMONITOR, MONITORINFOEXW)>) };
        let mut mi = MONITORINFOEXW::default();
        mi.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
        if unsafe { GetMonitorInfoW(m, &mut mi as *mut _ as *mut MONITORINFO) }.as_bool() { v.push((m, mi)); }
        BOOL(1)
    }
    let mut v: Vec<(HMONITOR, MONITORINFOEXW)> = vec![];
    unsafe { let _ = EnumDisplayMonitors(None, None, Some(cb), LPARAM(&mut v as *mut _ as isize)); }
    v.sort_by_key(|(_, mi)| (mi.monitorInfo.rcMonitor.left, mi.monitorInfo.rcMonitor.top));
    let mut k = 1.0;
    let e = v.iter().enumerate().map(|(i, (m, mi))| {
        let (r, t) = (mi.monitorInfo.rcMonitor, mi.monitorInfo.rcWork);
        let principal = mi.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0;
        if principal {
            let (mut dx, mut dy) = (96u32, 96u32);
            if unsafe { GetDpiForMonitor(*m, MDT_EFFECTIVE_DPI, &mut dx, &mut dy) }.is_ok() { k = dx as f32 / 96.0; }
        }
        let n = mi.szDevice.iter().position(|&c| c == 0).unwrap_or(0);
        Ecran {
            id: i, nom: String::from_utf16_lossy(&mi.szDevice[..n]).replace("\\\\.\\", ""),
            x: r.left as f32, y: r.top as f32, r: r.right as f32, b: r.bottom as f32,
            sol: t.bottom as f32, gx: t.left as f32, dx: t.right as f32, principal,
        }
    }).collect();
    (e, k)
}

/// Boîte de message d'erreur (avant que les fenêtres ne soient prêtes).
pub fn alerte(texte: &str) {
    let t = wstr(texte);
    unsafe { MessageBoxW(None, windows::core::PCWSTR(t.as_ptr()), w!("PixelPal"), MB_ICONWARNING | MB_OK); }
}

pub fn curseur() -> (f32, f32) {
    let mut p = POINT::default();
    unsafe { let _ = GetCursorPos(&mut p); }
    (p.x as f32, p.y as f32)
}

/// Tête de l'aventurier (chapeau compris), en icône 32 × 32.
fn icone_perso() -> Option<HICON> {
    let img = crate::sprites::Sprites::default().image_n("aventurier", "idle", 0).clone();
    let mut t = Toile::new(32, 32);
    t.coller(&img, -2.0, -3.0, 2.0, false, 1.0); // cadrage sur la tête (lignes 2 à 17 du sprite)
    unsafe {
        let ecran = GetDC(None);
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER { biSize: size_of::<BITMAPINFOHEADER>() as u32, biWidth: 32, biHeight: -32, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, ..Default::default() },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let couleur = CreateDIBSection(Some(ecran), &bmi, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        ReleaseDC(None, ecran);
        let dst = std::slice::from_raw_parts_mut(bits as *mut u8, 32 * 32 * 4);
        for (i, &c) in t.px.iter().enumerate() {
            dst[i * 4..i * 4 + 4].copy_from_slice(&[c as u8, (c >> 8) as u8, (c >> 16) as u8, (c >> 24) as u8]);
        }
        let masque = CreateBitmap(32, 32, 1, 1, Some([0u8; 128].as_ptr() as *const c_void));
        let info = ICONINFO { fIcon: true.into(), xHotspot: 0, yHotspot: 0, hbmMask: masque, hbmColor: couleur };
        let h = CreateIconIndirect(&info).ok();
        let _ = DeleteObject(couleur.into());
        let _ = DeleteObject(masque.into());
        h
    }
}

/// Icône dans la zone de notification : clic gauche = panneau, clic droit = menu (panneau, quitter).
pub struct Icone { pub hwnd: HWND }

impl Icone {
    pub fn new() -> Icone {
        unsafe {
            let hwnd = CreateWindowExW(WS_EX_TOOLWINDOW, w!("PixelPalCalque"), w!("PixelPal icône"), WS_POPUP, 0, 0, 0, 0, None, None,
                Some(GetModuleHandleW(None).unwrap().into()), None).expect("fenêtre icône");
            let mut nid = NOTIFYICONDATAW {
                cbSize: size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd, uID: 1, uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
                uCallbackMessage: WM_ICONE, hIcon: icone_perso().unwrap_or_else(|| LoadIconW(None, IDI_APPLICATION).unwrap_or_default()), ..Default::default()
            };
            let tip = wstr("PixelPal : clic pour ouvrir le panneau");
            nid.szTip[..tip.len()].copy_from_slice(&tip);
            let _ = Shell_NotifyIconW(NIM_ADD, &nid);
            Icone { hwnd }
        }
    }
}

impl Drop for Icone {
    fn drop(&mut self) {
        unsafe {
            let nid = NOTIFYICONDATAW { cbSize: size_of::<NOTIFYICONDATAW>() as u32, hWnd: self.hwnd, uID: 1, ..Default::default() };
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

/// Traite les messages Windows en attente ; renvoie faux si l'application doit s'arrêter.
pub fn pomper() -> bool {
    unsafe {
        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            if msg.message == WM_QUIT { return false; }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    true
}
