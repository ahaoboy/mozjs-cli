/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this file,
 * You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::ffi::CStr;
use std::ptr;
use std::str;

use mozjs::glue::EncodeStringToUTF8;
use mozjs::jsapi::{CallArgs, JSAutoRealm, JSContext, OnNewGlobalHookOption, Value};
use mozjs::jsapi::{JS_DefineFunction, JS_NewGlobalObject, JS_ReportErrorASCII};
use mozjs::jsval::UndefinedValue;
use mozjs::rooted;
use mozjs::rust::{JSEngine, RealmOptions, Runtime, SIMPLE_GLOBAL_CLASS};

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        println!("usage: mozjs <FILE.js>");
        return;
    };

    let Ok(javascript) = std::fs::read_to_string(&path) else {
        println!("failed to read {path}");
        return;
    };

    let engine = JSEngine::init().unwrap();
    let runtime = Runtime::new(engine.handle());
    let context = runtime.cx();
    let h_option = OnNewGlobalHookOption::FireOnNewGlobalHook;
    let c_option = RealmOptions::default();

    unsafe {
        rooted!(in(context) let global = JS_NewGlobalObject(
            context,
            &SIMPLE_GLOBAL_CLASS,
            ptr::null_mut(),
            h_option,
            &*c_option,
        ));
        let _ac = JSAutoRealm::new(context, global.get());

        let function = JS_DefineFunction(
            context,
            global.handle().into(),
            c"print".as_ptr(),
            Some(print),
            1,
            0,
        );
        assert!(!function.is_null());

        rooted!(in(context) let mut rval = UndefinedValue());
        assert!(
            runtime
                .evaluate_script(
                    global.handle(),
                    &javascript,
                    "test.js",
                    0,
                    rval.handle_mut()
                )
                .is_ok()
        );
    }
}

unsafe extern "C" fn print(context: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
    unsafe {
        let args = CallArgs::from_vp(vp, argc);

        if args.argc_ != 1 {
            JS_ReportErrorASCII(context, c"print() requires exactly 1 argument".as_ptr());
            return false;
        }

        let arg = mozjs::rust::Handle::from_raw(args.get(0));
        let js = mozjs::rust::ToString(context, arg);
        rooted!(in(context) let message_root = js);
        unsafe extern "C" fn cb(message: *const core::ffi::c_char) {
            unsafe {
                let message = CStr::from_ptr(message);
                let message = str::from_utf8(message.to_bytes()).unwrap();
                println!("{}", message);
            }
        }
        EncodeStringToUTF8(context, message_root.handle().into(), cb);

        args.rval().set(UndefinedValue());
        true
    }
}
