#[macro_use]
extern crate lazy_static;

use auxtools::*;

use coarsetime::{Duration, Instant};

type DeferredFunc = Box<dyn Fn() -> DMResult + Send + Sync>;

type CallbackChannel = (flume::Sender<DeferredFunc>, flume::Receiver<DeferredFunc>);

lazy_static! {
    static ref CALLBACK_CHANNEL: CallbackChannel = flume::bounded(100000);
}

#[shutdown]
fn _clean_callbacks() {
    for cb in CALLBACK_CHANNEL.1.try_iter() {
        drop(cb);
    }
}

/// This gives you a copy of the callback sender. Send to it with try_send or send, then later it'll be processed
/// if one of the process_callbacks functions is called for any reason.
pub fn byond_callback_sender() -> flume::Sender<DeferredFunc> {
    CALLBACK_CHANNEL.0.clone()
}

/// Goes through every single outstanding callback and calls them.
pub fn process_callbacks() {
    let stack_trace = Proc::find("/proc/auxtools_stack_trace").unwrap();
    for callback in CALLBACK_CHANNEL.1.try_iter() {
        if let Err(e) = callback() {
            let _ = stack_trace.call(&[&Value::from_string(e.message.as_str()).unwrap()]);
        }
        drop(callback);
    }
}

/// Goes through every single outstanding callback and calls them, until a given time limit is reached.
pub fn process_callbacks_for(duration: Duration) -> bool {
    let now = Instant::now();
    let stack_trace = Proc::find("/proc/auxtools_stack_trace").unwrap();
    for callback in CALLBACK_CHANNEL.1.try_iter() {
        if let Err(e) = callback() {
            let _ = stack_trace.call(&[&Value::from_string(e.message.as_str()).unwrap()]);
        }
        drop(callback);
        if now.elapsed() > duration {
            break;
        }
    }
    now.elapsed() > duration
}

/// Goes through every single outstanding callback and calls them, until a given time limit in milliseconds is reached.
pub fn process_callbacks_for_millis(millis: u64) -> bool {
    process_callbacks_for(Duration::from_millis(millis))
}

/// This function is to be called from byond, preferably once a tick.
/// Calling with no arguments will process every outstanding callback.
/// Calling with one argument will process the callbacks until a given time limit is reached.
/// Time limit is in milliseconds.
/// This has to be manually hooked in the code, e.g.
/// ```
/// #[hook("/proc/process_atmos_callbacks")]
/// fn _atmos_callback_handle() {
///     auxcallback::callback_processing_hook(args)
/// }
/// ```

pub fn callback_processing_hook(args: &mut Vec<Value>) -> DMResult {
    match args.len() {
        0 => {
            process_callbacks();
            Ok(Value::null())
        }
        _ => {
            let arg_limit = args.get(0).unwrap().as_number()? as u64;
            Ok(Value::from(process_callbacks_for_millis(arg_limit)))
        }
    }
}
