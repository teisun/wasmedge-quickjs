//! pi-rust-wasm host call bridge: imports `env.__pi_host_call` from the host
//! and exposes it as a global JS function `__pi_host_call(requestJson) -> responseJson`.

use wasmedge_quickjs::{AsObject, Context, JsFn, JsValue};

const MIN_BUF_CAP: usize = 64 * 1024; // 64 KB minimum response buffer

#[link(wasm_import_module = "env")]
extern "C" {
    /// Host-provided function: reads req_len bytes of request JSON from buf_ptr,
    /// writes response JSON back to buf_ptr (up to buf_cap bytes),
    /// returns actual response length.
    fn __pi_host_call(buf_ptr: i32, req_len: i32, buf_cap: i32) -> i32;
}

pub struct PiHostCallFn;

impl JsFn for PiHostCallFn {
    fn call(ctx: &mut Context, _this_val: JsValue, argv: &[JsValue]) -> JsValue {
        let request = match argv.get(0) {
            Some(JsValue::String(s)) => s.to_string(),
            _ => return ctx.throw_type_error("__pi_host_call expects a string argument").into(),
        };

        let req_bytes = request.as_bytes();
        let buf_cap = std::cmp::max(req_bytes.len() * 4, MIN_BUF_CAP);
        let mut buf = vec![0u8; buf_cap];
        buf[..req_bytes.len()].copy_from_slice(req_bytes);

        let out_len = unsafe {
            __pi_host_call(
                buf.as_mut_ptr() as i32,
                req_bytes.len() as i32,
                buf_cap as i32,
            )
        };

        if out_len <= 0 {
            return ctx.throw_type_error("__pi_host_call: host returned empty or error").into();
        }

        let out_len = out_len as usize;

        if out_len > buf_cap {
            // Response didn't fit; retry with a larger buffer.
            let bigger_cap = out_len;
            let mut bigger_buf = vec![0u8; bigger_cap];
            bigger_buf[..req_bytes.len()].copy_from_slice(req_bytes);
            let out_len2 = unsafe {
                __pi_host_call(
                    bigger_buf.as_mut_ptr() as i32,
                    req_bytes.len() as i32,
                    bigger_cap as i32,
                )
            };
            let out_len2 = out_len2 as usize;
            if out_len2 > bigger_cap || out_len2 == 0 {
                return ctx
                    .throw_type_error("__pi_host_call: response too large after retry")
                    .into();
            }
            match std::str::from_utf8(&bigger_buf[..out_len2]) {
                Ok(s) => return ctx.new_string(s).into(),
                Err(_) => {
                    return ctx
                        .throw_type_error("__pi_host_call: invalid UTF-8 in response")
                        .into()
                }
            }
        }

        match std::str::from_utf8(&buf[..out_len]) {
            Ok(s) => ctx.new_string(s).into(),
            Err(_) => ctx
                .throw_type_error("__pi_host_call: invalid UTF-8 in response")
                .into(),
        }
    }
}

/// Register `__pi_host_call` as a global JS function in the given QuickJS context.
pub fn register_pi_host_call(ctx: &mut Context) {
    let f = ctx.new_function::<PiHostCallFn>("__pi_host_call");
    ctx.get_global().set("__pi_host_call", f.into());
}
