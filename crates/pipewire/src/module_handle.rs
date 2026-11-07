/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use pipewire_sys::pw_impl_module;

#[derive(Debug)]
pub(crate) struct ModuleHandle(*mut pw_impl_module);

impl ModuleHandle {
    pub(crate) fn new(ptr: *mut pw_impl_module) -> Self {
        Self(ptr)
    }
}

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
