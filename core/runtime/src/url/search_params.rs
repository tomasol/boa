//! Boa's implementation of JavaScript's `URLSearchParams` Web API class.
//!
//! More information:
//!  - [MDN documentation][mdn]
//!  - [WHATWG `URL` specification][spec]
//!
//! [spec]: https://url.spec.whatwg.org/#interface-urlsearchparams
//! [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/URLSearchParams
#![allow(clippy::needless_pass_by_value)]

use boa_engine::interop::JsClass;
use boa_engine::object::builtins::{JsArray, TypedJsFunction};
use boa_engine::realm::Realm;
use boa_engine::value::{Convert, TryFromJs};
use boa_engine::{
    Context, Finalize, JsData, JsObject, JsResult, JsString, JsValue, Trace, boa_class, boa_module,
    js_error,
};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// Callback type for [`UrlSearchParams::for_each`].
type ForEachCallback = TypedJsFunction<(JsString, JsString, JsObject), ()>;

/// The `URLSearchParams` class provides utility methods to work with the query string
/// of a URL.
///
/// When created via [`UrlSearchParams::from_url`], mutations are synced back to the
/// owning URL's query string. Note that mutations to `URL.search` after calling
/// `URL.searchParams` will not update the already-returned params object (spec TODO).
#[derive(Debug, Clone, JsData, Trace, Finalize)]
pub struct UrlSearchParams {
    #[unsafe_ignore_trace]
    params: Rc<RefCell<Vec<(String, String)>>>,
    /// Optional back-reference to the owning URL. When `Some`, every mutation
    /// is serialized back to the URL's query string.
    #[unsafe_ignore_trace]
    url_sync: Option<Rc<RefCell<url::Url>>>,
}

impl UrlSearchParams {
    /// Register the `URLSearchParams` class into the realm.
    ///
    /// # Errors
    /// This will error if the context or realm cannot register the class.
    pub fn register(realm: Option<Realm>, context: &mut Context) -> JsResult<()> {
        js_module::boa_register(realm, context)
    }

    /// Creates a [`UrlSearchParams`] linked to an existing `url::Url`.
    ///
    /// The initial params are parsed from the URL's current query string.
    /// Every subsequent mutation of the params is synced back to the URL.
    pub(super) fn from_url(url: Rc<RefCell<url::Url>>) -> Self {
        let params = url
            .borrow()
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect::<Vec<_>>();
        Self {
            params: Rc::new(RefCell::new(params)),
            url_sync: Some(url),
        }
    }

    /// Serializes the current params back to the linked URL's query string, if any.
    fn sync_to_url(&self) {
        if let Some(url_ref) = &self.url_sync {
            let params = self.params.borrow();
            let mut url = url_ref.borrow_mut();
            if params.is_empty() {
                url.set_query(None);
            } else {
                let query = url::form_urlencoded::Serializer::new(String::new())
                    .extend_pairs(params.iter())
                    .finish();
                url.set_query(Some(&query));
            }
        }
    }
}

#[boa_class(rename = "URLSearchParams")]
#[boa(rename_all = "camelCase")]
impl UrlSearchParams {
    /// Create a new `URLSearchParams` object.
    ///
    /// `init` may be:
    /// - A string like `"key=value&key2=value2"` (a leading `?` is stripped),
    /// - An array of `[name, value]` string pairs,
    /// - A record (plain object) with string values, or
    /// - Another `URLSearchParams` instance.
    ///
    /// # Errors
    /// Returns a `TypeError` if `init` cannot be converted to params.
    #[boa(constructor)]
    pub fn new(init: JsValue, context: &mut Context) -> JsResult<Self> {
        if init.is_undefined() || init.is_null() {
            return Ok(Self {
                params: Rc::new(RefCell::new(Vec::new())),
                url_sync: None,
            });
        }

        // For object types (arrays, records, URLSearchParams) we must check
        // before attempting string coercion, since arrays coerce to strings like
        // "k1,v1,k2,v2" which would silently parse incorrectly.
        if init.is_object() {
            // Copy from another URLSearchParams instance.
            if let Some(other) = init
                .as_object()
                .as_ref()
                .and_then(JsObject::downcast_ref::<UrlSearchParams>)
            {
                return Ok(Self {
                    params: Rc::new(RefCell::new(other.params.borrow().clone())),
                    url_sync: None,
                });
            }

            // Parse from an array of `[name, value]` pairs.
            if let Ok(pairs) = Vec::<Vec<String>>::try_from_js(&init, context) {
                let params = pairs
                    .into_iter()
                    .map(|pair| {
                        if pair.len() == 2 {
                            Ok((pair[0].clone(), pair[1].clone()))
                        } else {
                            Err(js_error!(TypeError: "Each init sequence must have exactly 2 elements"))
                        }
                    })
                    .collect::<JsResult<Vec<_>>>()?;
                return Ok(Self {
                    params: Rc::new(RefCell::new(params)),
                    url_sync: None,
                });
            }

            // Parse from a record object `{name: value, ...}`.
            if let Ok(record) = BTreeMap::<String, Convert<String>>::try_from_js(&init, context) {
                let params = record
                    .into_iter()
                    .map(|(k, v)| (k, v.0.clone()))
                    .collect::<Vec<_>>();
                return Ok(Self {
                    params: Rc::new(RefCell::new(params)),
                    url_sync: None,
                });
            }
        }

        // For strings and other non-object types, coerce to string and parse as
        // an application/x-www-form-urlencoded query string (stripping a leading `?`).
        if let Ok(Convert(ref s)) = Convert::<String>::try_from_js(&init, context) {
            let query = s.trim_start_matches('?');
            let params = url::form_urlencoded::parse(query.as_bytes())
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect::<Vec<_>>();
            return Ok(Self {
                params: Rc::new(RefCell::new(params)),
                url_sync: None,
            });
        }

        Err(js_error!(TypeError: "URLSearchParams init must be a string, sequence of pairs, record, or URLSearchParams"))
    }

    /// Appends a new name/value pair to the list.
    pub fn append(&mut self, name: Convert<String>, value: Convert<String>) {
        self.params
            .borrow_mut()
            .push((name.0.clone(), value.0.clone()));
        self.sync_to_url();
    }

    /// Removes all entries whose name equals `name`. If `value` is provided,
    /// only entries matching both name and value are removed.
    pub fn delete(&mut self, name: Convert<String>, value: Option<Convert<String>>) {
        match &value {
            Some(v) => {
                let v_str = v.0.as_str();
                self.params
                    .borrow_mut()
                    .retain(|(k, val)| !(k.as_str() == name.0.as_str() && val.as_str() == v_str));
            }
            None => {
                let name_str = name.0.as_str();
                self.params
                    .borrow_mut()
                    .retain(|(k, _)| k.as_str() != name_str);
            }
        }
        self.sync_to_url();
    }

    /// Returns an array of all `[name, value]` pairs in insertion order.
    pub fn entries(&self, context: &mut Context) -> JsValue {
        JsArray::from_iter(
            self.params
                .borrow()
                .iter()
                .map(|(k, v)| {
                    let k: JsValue = JsString::from(k.as_str()).into();
                    let v: JsValue = JsString::from(v.as_str()).into();
                    JsArray::from_iter([k, v], context).into()
                })
                .collect::<Vec<_>>(),
            context,
        )
        .into()
    }

    /// Calls `callbackFn(value, name, searchParams)` for each name/value pair.
    ///
    /// # Errors
    /// Propagates any error thrown by `callbackFn`.
    #[allow(clippy::needless_pass_by_value)]
    #[boa(method)]
    pub fn for_each(
        this: JsClass<Self>,
        callback: ForEachCallback,
        this_arg: Option<JsValue>,
        context: &mut Context,
    ) -> JsResult<()> {
        let object = this.inner().upcast();
        let this_arg = this_arg.unwrap_or_default();
        // Snapshot the params so the callback can mutate the params list.
        let pairs: Vec<(String, String)> = this.clone_inner().params.borrow().clone();
        for (k, v) in pairs {
            callback.call_with_this(
                &this_arg,
                context,
                (
                    JsString::from(v.as_str()),
                    JsString::from(k.as_str()),
                    object.clone(),
                ),
            )?;
        }
        Ok(())
    }

    /// Returns the first value associated with `name`, or `null` if not found.
    pub fn get(&self, name: Convert<String>) -> JsValue {
        self.params
            .borrow()
            .iter()
            .find(|(k, _)| k.as_str() == name.0.as_str())
            .map_or(JsValue::null(), |(_, v)| JsString::from(v.as_str()).into())
    }

    /// Returns all values associated with `name` as an array.
    pub fn get_all(&self, name: Convert<String>) -> Vec<JsString> {
        self.params
            .borrow()
            .iter()
            .filter(|(k, _)| k.as_str() == name.0.as_str())
            .map(|(_, v)| JsString::from(v.as_str()))
            .collect()
    }

    /// Returns `true` if at least one entry with the given `name` exists.
    /// If `value` is also provided, both name and value must match.
    pub fn has(&self, name: Convert<String>, value: Option<Convert<String>>) -> bool {
        let params = self.params.borrow();
        match &value {
            Some(v) => params
                .iter()
                .any(|(k, val)| k.as_str() == name.0.as_str() && val.as_str() == v.0.as_str()),
            None => params.iter().any(|(k, _)| k.as_str() == name.0.as_str()),
        }
    }

    /// Returns an array of all names in insertion order.
    pub fn keys(&self) -> Vec<JsString> {
        self.params
            .borrow()
            .iter()
            .map(|(k, _)| JsString::from(k.as_str()))
            .collect()
    }

    /// Sets the value of `name` to `value`. If there are multiple entries with
    /// `name`, the first is updated and the rest are removed.
    pub fn set(&mut self, name: Convert<String>, value: Convert<String>) {
        let name_str = name.0.clone();
        let value_str = value.0.clone();
        let mut params = self.params.borrow_mut();
        let mut found = false;
        params.retain_mut(|(k, v)| {
            if k.as_str() == name_str.as_str() {
                if !found {
                    found = true;
                    *v = value_str.clone();
                    true
                } else {
                    false
                }
            } else {
                true
            }
        });
        if !found {
            params.push((name_str, value_str));
        }
        drop(params);
        self.sync_to_url();
    }

    /// Returns the number of name/value pairs.
    #[boa(getter)]
    pub fn size(&self) -> usize {
        self.params.borrow().len()
    }

    /// Sorts all entries in-place by name in ascending Unicode code-unit order.
    /// The relative order of entries with equal names is preserved.
    pub fn sort(&mut self) {
        self.params
            .borrow_mut()
            .sort_by(|(a, _), (b, _)| a.cmp(b));
        self.sync_to_url();
    }

    /// Returns the serialized form: `"name=value&name2=value2"`.
    fn to_string(&self) -> JsString {
        JsString::from(
            url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(self.params.borrow().iter())
                .finish(),
        )
    }

    /// Returns an array of all values in insertion order.
    pub fn values(&self) -> Vec<JsString> {
        self.params
            .borrow()
            .iter()
            .map(|(_, v)| JsString::from(v.as_str()))
            .collect()
    }
}

/// JavaScript module containing the `URLSearchParams` class.
#[boa_module]
pub mod js_module {
    type UrlSearchParams = super::UrlSearchParams;
}
