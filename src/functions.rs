use crate::{error::MapEdiError, exp_errf};
use boa_engine::{js_string, Context, JsError, JsNativeError, JsResult, JsValue, NativeFunction};
use chrono::DateTime;

macro_rules! js_err {
    ($m:expr) => {
        |_| JsError::from_native(JsNativeError::error().with_message($m))
    };
}

pub fn register_functions_default(context: &mut Context) -> Result<(), MapEdiError> {
    #[inline]
    pub fn register_callback<F>(
        context: &mut Context,
        name: &str,
        len: usize,
        f: F,
    ) -> Result<(), MapEdiError>
    where
        F: Fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> + Copy + 'static,
    {
        context
            .register_global_callable(js_string!(name), len, NativeFunction::from_copy_closure(f))
            .map_err(exp_errf!(name))?;

        Ok(())
    }

    register_callback(context, "$dateHHMM", 1, date_hhmm)?;
    register_callback(context, "$dateHHMMSSSS", 1, todo)?;
    register_callback(context, "$dateYYYYMMDD", 1, todo)?;
    register_callback(context, "$dateYYMMDD", 1, date_yymmdd)?;
    register_callback(context, "$dateFmt", 1, date_refmt)?;
    register_callback(context, "$dateRefmt", 1, date_refmt)?;
    register_callback(context, "$fallback", 1, fallback)?;
    register_callback(context, "$hl", 1, todo)?;
    register_callback(context, "$hlParent", 1, todo)?;
    register_callback(context, "$st02", 1, todo)?;
    register_callback(context, "$se01", 1, todo)?;
    register_callback(context, "$se02", 1, todo)?;
    register_callback(context, "$zip9", 1, todo)?;
    register_callback(context, "$pad", 4, pad)?;
    register_callback(context, "print", 1, print)?;
    register_callback(context, "$stripPunc", 1, todo)?;
    register_callback(context, "$middleName", 1, todo)?;
    register_callback(context, "$firstName", 1, todo)?;
    register_callback(context, "$lastName", 1, todo)?;
    Ok(())
}

pub fn register_extra<F>(
    include_defaults: bool,
    fs: Vec<(String, usize, F)>,
) -> impl Fn(&mut Context) -> Result<(), MapEdiError>
where
    F: Fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> + Copy + 'static,
{
    move |context: &mut Context| {
        if include_defaults {
            register_functions_default(context)?;
        }

        for (name, length, body) in fs.clone().into_iter() {
            context
                .register_global_callable(
                    js_string!(name.clone()),
                    length,
                    NativeFunction::from_copy_closure(body),
                )
                .map_err(exp_errf!(name))?;
        }

        Ok(())
    }
}

#[allow(dead_code)]
fn date_yyyymmdd(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let iso_date = match args.iter().nth(0) {
        Some(iso_date) => match iso_date {
            JsValue::String(iso_date) => iso_date
                .to_std_string()
                .map_err(|_| JsError::from_native(JsNativeError::error()))?,
            _ => return Ok(JsValue::Null),
        },
        None => return Ok(JsValue::Null),
    };

    let yyyymmdd_date = DateTime::parse_from_rfc3339(&iso_date)
        .map(|date| date.format("%Y%m%d").to_string())
        .map_err(|_| JsError::from_native(JsNativeError::error().with_message("not ISO-8601")))?;

    Ok(JsValue::String(js_string!(yyyymmdd_date)))
}

#[allow(dead_code)]
fn date_yymmdd(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let iso_date = match args.iter().nth(0) {
        Some(iso_date) => match iso_date {
            JsValue::String(iso_date) => iso_date
                .to_std_string()
                .map_err(|_| JsError::from_native(JsNativeError::error()))?,
            _ => return Ok(JsValue::Null),
        },
        None => return Ok(JsValue::Null),
    };

    let yyyymmdd_date = DateTime::parse_from_rfc3339(&iso_date)
        .map(|date| date.format("%C%m%d").to_string())
        .map_err(|_| JsError::from_native(JsNativeError::error().with_message("not ISO-8601")))?;

    Ok(JsValue::String(js_string!(yyyymmdd_date)))
}

#[allow(dead_code)]
fn date_hhmm(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let iso_date = match args.iter().nth(0) {
        Some(iso_date) => match iso_date {
            JsValue::String(iso_date) => iso_date
                .to_std_string()
                .map_err(|_| JsError::from_native(JsNativeError::error()))?,
            x => {
                return Ok(JsValue::Null);
            }
        },
        None => return Ok(JsValue::Null),
    };

    let yyyymmdd_date = DateTime::parse_from_rfc3339(&iso_date)
        .map(|date| date.format("%H%M").to_string())
        .map_err(|_| JsError::from_native(JsNativeError::error().with_message("not ISO-8601")))?;

    Ok(JsValue::String(js_string!(yyyymmdd_date)))
}

#[allow(dead_code)]
fn date_hhmmssss(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let iso_date = match args.iter().nth(0) {
        Some(iso_date) => match iso_date {
            JsValue::String(iso_date) => iso_date
                .to_std_string()
                .map_err(|_| JsError::from_native(JsNativeError::error()))?,
            _ => return Ok(JsValue::Null),
        },
        None => return Ok(JsValue::Null),
    };

    let yyyymmdd_date = DateTime::parse_from_rfc3339(&iso_date)
        .map(|date| date.format("%H%M%S00").to_string())
        .map_err(js_err!("not ISO-8601"))?;

    Ok(JsValue::String(js_string!(yyyymmdd_date)))
}

fn date_refmt(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    // let args = args.iter();
    // let inp = if let Some(x) = args.next().unwrap().as_string() {
    //     x.to_std_string()
    //         .map_err(js_err!("inp not stringy enough"))?;
    // } else {
    //     return Ok(JsValue::null());
    // };
    // let pat = args.next().unwrap().as_string();
    // let outpat = args.next().unwrap().as_string();

    Ok(JsValue::String(js_string!("todo!")))
}

fn fallback(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let result = args
        .to_vec()
        .into_iter()
        .find(|e| !matches!(e, JsValue::Null | JsValue::Undefined))
        .unwrap_or(JsValue::Null);
    Ok(result)
}

fn todo(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::String(js_string!("todo!")))
}

fn pad(_: &JsValue, _args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::String(js_string!("todo!")))
}

fn print(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let json = context
        .global_object()
        .get(js_string!("JSON"), context)
        .unwrap();

    let stringify = json
        .as_object()
        .unwrap()
        .get(js_string!("stringify"), context)?;

    let value = stringify
        .as_callable()
        .unwrap()
        .call(&json, args, context)
        .map(|x| x.to_string(context))?
        .unwrap()
        .to_std_string()
        .unwrap();

    println!("{}", value);

    Ok(JsValue::undefined())
}
