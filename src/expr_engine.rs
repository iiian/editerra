use boa_engine::{js_string, property::Attribute, Context, JsValue, Source};

use crate::{error::MapEdiError, exp_errf};

/// An expression engine built on top of mlua
#[derive(Default)]
pub struct ExprEngine {
    boa: boa_engine::Context,
}

impl ExprEngine {
    pub fn init_with_haystack(&mut self, json_string: String) -> Result<(), MapEdiError> {
        let context = &mut self.boa;
        let json = context
            .global_object()
            .get(js_string!("JSON"), context)
            .map_err(|e| MapEdiError::ExprEngineErr(e.to_string()))?;

        let parse = json
            .as_object()
            .unwrap()
            .get(js_string!("parse"), context)
            .map_err(|e| MapEdiError::ExprEngineErr(e.to_string()))?;

        let js_string = js_string!(json_string);

        let value = parse
            .as_callable()
            .unwrap()
            .call(&json, &[JsValue::new(js_string)], context)
            .map_err(|e| MapEdiError::ExprEngineErr(e.to_string()))?;

        // var $$ = <value>;
        self.boa
            .register_global_property(js_string!("$$"), value, Attribute::all())
            .map_err(|e| MapEdiError::ExprEngineErr(e.to_string()))?;
        self.boa
            .eval(Source::from_bytes(&String::from("var $ = $$;")))
            .map_err(exp_errf!())?;

        // var ctx = [];
        let array_constructor = self
            .boa
            .global_object()
            .get(js_string!("Array"), &mut self.boa)
            .map_err(exp_errf!())?;
        let js_array = array_constructor
            .as_constructor()
            .unwrap()
            .construct(&[], None, &mut self.boa)
            .map_err(exp_errf!())?;
        self.boa
            .register_global_property(js_string!("ctx"), js_array, Attribute::all())
            .map_err(exp_errf!())?;
        let js_array = array_constructor
            .as_constructor()
            .unwrap()
            .construct(&[], None, &mut self.boa)
            .map_err(exp_errf!())?;
        self.boa
            .register_global_property(js_string!("iter"), js_array, Attribute::all())
            .map_err(exp_errf!())?;

        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), MapEdiError> {
        self.boa
            .global_object()
            .delete_property_or_throw(js_string!("$$"), &mut self.boa)
            .map_err(|e| MapEdiError::ExprEngineErr(e.to_string()))?;
        self.boa
            .global_object()
            .delete_property_or_throw(js_string!("$"), &mut self.boa)
            .map_err(|e| MapEdiError::ExprEngineErr(e.to_string()))?;
        self.boa
            .global_object()
            .delete_property_or_throw(js_string!("ctx"), &mut self.boa)
            .map_err(|e| MapEdiError::ExprEngineErr(e.to_string()))?;

        Ok(())
    }

    pub fn register_functions<F>(&mut self, register_callback: F) -> Result<(), MapEdiError>
    where
        F: Fn(&mut Context) -> Result<(), MapEdiError>,
    {
        register_callback(&mut self.boa)?;

        Ok(())
    }

    pub fn eval(&mut self, expr_raw: &String) -> Result<JsValue, MapEdiError> {
        self.boa
            .eval(Source::from_bytes(expr_raw))
            .map_err(|e| MapEdiError::ExprEngineErr(e.to_string()))
    }

    pub fn eval_bool(&mut self, expr_raw: &String) -> Result<bool, MapEdiError> {
        self.boa
            .eval(Source::from_bytes(expr_raw))
            .map(|x| match x {
                JsValue::Null | JsValue::Undefined => false,
                JsValue::Boolean(x) => x,
                JsValue::String(s) => s.len() > 0,
                JsValue::Rational(r) => r != 0.0,
                JsValue::Integer(i) => i != 0,
                JsValue::BigInt(b) => b != 0,
                JsValue::Object(_) => true,
                JsValue::Symbol(_) => true,
            })
            .map_err(exp_errf!())
    }

    pub fn eval_string(&mut self, expr_raw: &String) -> Result<Option<String>, MapEdiError> {
        self.boa
            .eval(Source::from_bytes(expr_raw))
            .map(|x| match x {
                JsValue::Null | JsValue::Undefined => None,
                JsValue::Boolean(k) => Some(k.to_string()),
                JsValue::String(s) => Some(s.to_std_string().unwrap()),
                JsValue::Rational(r) => Some(r.to_string()),
                JsValue::Integer(i) => Some(i.to_string()),
                JsValue::BigInt(b) => Some(b.to_string()),
                JsValue::Object(x) => Some("[Object object]".to_string()),
                JsValue::Symbol(_) => Some("[Symbol]".to_string()),
            })
            .map_err(exp_errf!())
    }

    pub fn exec(&mut self, expr_raw: &String) -> Result<(), MapEdiError> {
        self.boa
            .eval(Source::from_bytes(expr_raw))
            .map_err(exp_errf!())?;

        Ok(())
    }
}
