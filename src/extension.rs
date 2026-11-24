// Copyright (c) 2026 Project Contributors
//
// Licensed under either of the following licenses, at your option:
//
// 1. The Apache License, Version 2.0
//    (https://www.apache.org/licenses/LICENSE-2.0)
// 2. The MIT License
//    (https://opensource.org/license/mit)
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{error::Error, fs::read_dir, path::Path};

use home::home_dir;
use libloading::{Library, Symbol};
use safer_ffi::prelude::*;

use crate::interpreter::MODIFIERS;

pub type ModifierHandler = extern "C" fn(
    usize,
    c_slice::Ref<'static, char_p::Ref<'static>>,
    c_slice::Mut<'static, char_p::Box>,
) -> Option<char_p::Box>;

#[ffi_export]
fn add_argument_modifier(modifier: char_p::Ref<'_>, handler: ModifierHandler) {
    MODIFIERS.with_borrow_mut(|mods| mods.insert(modifier.to_string(), handler));
}

fn load(lib_path: &Path) -> Result<(), Box<dyn Error>> {
    unsafe {
        let lib = Library::new(lib_path)?;
        let init: Symbol<unsafe extern "C" fn(extern "C" fn(char_p::Ref<'_>, ModifierHandler))> =
            lib.get(b"init\0")?;

        init(add_argument_modifier);
    }

    Ok(())
}

pub fn load_all() -> Result<(), Box<dyn Error>> {
    let home = home_dir().ok_or::<Box<dyn Error>>("Cannot get home dir".into())?;

    for file in read_dir(home.join(".cytrick/ext"))? {
        let file = file?;

        match file.file_name().to_str() {
            Some(name) => println!("Loading extension {name}"),
            None => println!("Extension name is not valid UTF-8"),
        }

        load(&file.path())?;
    }

    Ok(())
}
