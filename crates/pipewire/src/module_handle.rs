/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use pipewire_sys::pw_impl_module;

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct ModuleHandle(pub(crate) *mut pw_impl_module);

// SAFETY: pw_impl_module is context-bound; sending the handle across threads
// requires holding the context lock or ensuring single-threaded event loop access.
#[allow(unsafe_code)]
unsafe impl Send for ModuleHandle {}
#[allow(unsafe_code)]
unsafe impl Sync for ModuleHandle {}

impl Drop for ModuleHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            #[expect(unsafe_code, reason = "FFI cleanup")]
            unsafe {
                pipewire_sys::pw_impl_module_destroy(self.0);
            }
        }
    }
}
